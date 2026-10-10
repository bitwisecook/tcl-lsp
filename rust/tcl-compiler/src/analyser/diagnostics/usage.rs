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

//! Usage and shape lint checks emitted during the command walk.
//!
//! These diagnostics inspect a single command's words and argument tokens
//! for style and shape problems that need no control- or data-flow:
//! unbraced `expr` / `if` / `while` bodies (W100, W105), unbraced `switch`
//! bodies (W106), a redundant nested `expr` (W114), the `string`-vs-`==`
//! comparison rewrite (W110), a `lappend` on a non-list value (W104),
//! name-vs-value confusion in a variable write (W212), a `${name}(...)`
//! array-write that should be `name(...)` (W216), non-ASCII and
//! encoding-mismatch text (W108, W311), invalid `binary format` modifiers
//! (W200), and an invalid subnet mask literal (W121).

use crate::optimiser::helpers::expr_simplify::eq_ne_compares_as_strings;
use rustc_hash::FxHashSet;
use tcl_core_types::DiagCode;
use tcl_lexer::SourceMap;

use super::helpers::{find_dotted_quads, source_slice};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use tcl_dialect::model::SpecProvider;

/// A proposal prepared from whole original expression words.
struct UnbracedSourceExpression {
    span: tcl_lexer::Span,
    text: String,
    has_sub: bool,
    fix_inner: Option<String>,
    subject: crate::analyser::DiagnosticSubject,
}

/// Original executable text parts supply lexical padding without materialising
/// substituted variables/commands or reparsing a reporting string.
fn original_word_has_pad_space(word: &tcl_lexer::NativeWord) -> bool {
    let arena = word.executable_parts();
    let parts = arena.list(arena.root());
    parts
        .first()
        .and_then(|part| arena.text(part))
        .is_some_and(|text| text.starts_with(b" "))
        || parts
            .last()
            .and_then(|part| arena.text(part))
            .is_some_and(|text| text.ends_with(b" "))
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
    /// Parser topology from the actual source input, independently of handler
    /// success. Complete availability validates the owner without choosing its
    /// expression or variable-reference grammar.
    fn original_expression_parser_context(
        &self,
    ) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
        let input = self.result.resolved_input.as_ref()?;
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            self.registry.as_deref()?,
            input,
            self.lexer_config(),
            self.unit_profile,
        )?;
        let mut parser = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .expression_parse_context(Some(input.unit_profile()));
        parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
        Some(parser)
    }

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
        // A whole-word command-substitution body — `eval [list set y $x]`
        // (the recommended *safe* form), `uplevel [buildScript]` — is
        // produced dynamically and parsed once by the consumer: there is
        // no double-substitution risk and it cannot be braced
        // (`eval {[list …]}` changes the meaning).  (A `Var` body such as
        // `while {$cond} $body` is *not* exempt — only a `Cmd` word is.)
        if matches!(body_tok.kind, tcl_lexer::TokenType::Cmd) {
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
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
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
            .with_fixes(vec![super::types::CodeFix {
                span: body_tok.span,
                new_text,
                description: "Wrap code block in braces".to_string(),
                // Per-instance: bracing a substitution-free body reaches the
                // command byte-identically; bracing one that substitutes
                // removes a round of substitution by design.
                safety: super::helpers::brace_wrap_fix_safety(trimmed, has_substitution),
            }]),
        );
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
        let safe = if expressions.concatenates {
            let context = self.original_expression_parser_context()?;
            is_safe_literal_expr_in_context(text.trim(), &context)
        } else {
            is_safe_literal(text.trim())
        };
        if safe {
            return None;
        }
        let has_sub = (argument..=last).any(|index| {
            original.word(index).is_some_and(|word| {
                word.executable_parts().all_parts().any(|part| {
                    matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Variable { .. }
                            | tcl_lexer::ExecutablePart::Command { .. }
                    )
                })
            })
        });
        let fix_inner = if argument == last {
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
        } = advice;
        let severity = if has_sub {
            Severity::Error
        } else {
            Severity::Warning
        };
        let message = if concatenates {
            "Expression is not braced: may cause double substitution and prevents byte-compilation. Use expr {...} instead.".to_owned()
        } else {
            format!(
                "Expression argument to '{command}' is not braced: may cause double substitution. Use braces: {{{text}}}"
            )
        };
        let fixes = fix_inner
            .into_iter()
            .map(|inner| super::types::CodeFix {
                span,
                new_text: format!("{{{inner}}}"),
                description: "Wrap expression in braces".to_owned(),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            })
            .collect();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::W100, span, message, severity)
                .with_fixes(fixes)
                .with_subject(subject),
        );
    }

    /// W311: a channel configured with `-encoding binary` *and*
    /// a non-binary `-translation` is contradictory (binary implies no
    /// translation) and can corrupt data / enable encoding-differential
    /// attacks.  Handles `fconfigure` and `chan configure`.
    pub(in crate::analyser) fn emit_w311_encoding_mismatch(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        // value-transfer-ok: options — W311 reads the encoding option's position, which `option_placement` on the registry will carry
        let opt_start = if cmd_name == "fconfigure" {
            1
            // value-transfer-ok: options — W311 reads the encoding option's position, which `option_placement` on the registry will carry
        } else if cmd_name == "chan" && args.first().map(String::as_str) == Some("configure") {
            2
        } else {
            return;
        };
        if args.len() <= opt_start {
            return;
        }
        let mut binary_tok = None;
        let mut translation_tok = None;
        let mut i = opt_start;
        while i + 1 < args.len() {
            let (opt, val) = (&args[i], &args[i + 1]);
            if opt == "-encoding" && val == "binary" {
                binary_tok = arg_tokens.get(i + 1);
            } else if opt == "-translation" && val != "binary" {
                translation_tok = arg_tokens.get(i + 1);
            }
            i += 2;
        }
        if binary_tok.is_some() && translation_tok.is_some() {
            let target = translation_tok
                .or(binary_tok)
                .or_else(|| arg_tokens.first());
            if let Some(tok) = target {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W311,
                        tok.span,
                        "Channel configured with -encoding binary and a non-binary \
                              -translation. Binary encoding implies no translation; the \
                              conflicting -translation may silently corrupt data or enable \
                              encoding-differential attacks."
                            .to_string(),
                        super::types::Severity::Warning,
                    ));
            }
        }
    }

    /// W200/W202: version gates on a literal `binary` template.
    ///
    /// Site selection uses the original source schema's shared
    /// `FormatType::Binary` projection. Effective alias ordinals retain their
    /// own written operand anchors; a captured prefix supplies no call-site
    /// span. Unknown template values and rebound heads supply no template.
    ///
    /// The field grammar itself comes from the binary owner
    /// (`tcl_cmd_core::binary::specifiers`), parsed with the suffix
    /// admitted so the gate — not the parse — decides; sites are
    /// buffered and settled post-walk against the effective Tcl version
    /// (§6 argument-DSL rung).
    ///
    /// W200 is the `u` suffix (TIP 275, Tcl 8.5). W202 is a field
    /// letter that does not exist on the target at all (`t n m r R q
    /// Q`, also 8.5); its floor comes from
    /// `tcl_cmd_core::binary::specifier_min_version`. They are separate
    /// codes because the fixes differ: a suffix can be dropped, an
    /// absent letter needs a different field.
    ///
    /// One site per code per template: every field shares the template
    /// token's span, so several gated fields give one squiggle each,
    /// not one per field.
    pub(in crate::analyser) fn emit_binary_field_version_gates(
        &mut self,
        templates: &[crate::analyser::commands::OriginalFormatTemplate],
    ) {
        for template in templates {
            if template.format.kind != tcl_registry::patterns::FormatType::Binary {
                continue;
            }
            self.push_binary_field_gates(&template.bytes, template.span);
        }
    }

    /// Buffer the W200/W202 gate sites for one literal template.
    fn push_binary_field_gates(&mut self, fmt: &[u8], span: tcl_lexer::Span) {
        let fields = tcl_cmd_core::binary::specifiers(fmt, true);
        if fields.iter().any(|f| f.modifier == Some(b'u')) {
            self.dsl_gate_sites.push(super::version_gate::DslGateSite {
                span,
                code: DiagCode::W200,
                what: "unsigned modifier 'u' on binary format specifier".to_string(),
                min: tcl_dialect::TclVersion::V8_5,
            });
        }

        // Gated field letters, deduped and reported in template order so
        // the message is stable regardless of how often each appears.
        let mut gated: Vec<(u8, tcl_dialect::TclVersion)> = Vec::new();
        for f in &fields {
            if let Some(min) = tcl_cmd_core::binary::specifier_min_version(f.letter)
                && !gated.iter().any(|(l, _)| *l == f.letter)
            {
                gated.push((f.letter, min));
            }
        }
        if let Some(min) = gated.iter().map(|(_, m)| *m).max() {
            let letters = gated
                .iter()
                .map(|(l, _)| format!("'{}'", char::from(*l)))
                .collect::<Vec<_>>()
                .join(", ");
            let noun = if gated.len() == 1 {
                "binary field specifier"
            } else {
                "binary field specifiers"
            };
            self.dsl_gate_sites.push(super::version_gate::DslGateSite {
                span,
                code: DiagCode::W202,
                what: format!("{noun} {letters}"),
                min,
            });
        }
    }

    /// W121: a dotted-quad literal that looks like a subnet mask
    /// but has non-contiguous bits (`255.255.255.1`, `255.0.255.0`) is
    /// almost certainly a mistake.
    pub(in crate::analyser) fn emit_w121_invalid_subnet_mask(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for (tok, text) in arg_tokens.iter().zip(args.iter()) {
            if seen.contains(&tok.span.start()) {
                continue;
            }
            for quad in find_dotted_quads(text, 3) {
                let octets: Vec<u32> = quad
                    .octets
                    .iter()
                    .map(|o| o.parse::<u32>().unwrap_or(999))
                    .collect();
                if octets.iter().any(|&o| o > 255) {
                    continue;
                }
                let (a, b, c, d) = (octets[0], octets[1], octets[2], octets[3]);
                if !looks_like_subnet_mask(a, b, c, d) {
                    continue;
                }
                if is_valid_subnet_mask(a, b, c, d) {
                    continue;
                }
                seen.insert(tok.span.start());
                let quad = format!("{a}.{b}.{c}.{d}");
                let mut message = format!(
                    "'{quad}' looks like a subnet mask but has non-contiguous bits. A valid \
                     mask must be contiguous leading 1-bits followed by 0-bits."
                );
                if let Some(s) = nearest_valid_mask(a, b, c, d)
                    && s != quad
                {
                    use std::fmt::Write as _;
                    let _ = write!(message, " Did you mean '{s}'?");
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W121,
                        tok.span,
                        message,
                        super::types::Severity::Warning,
                    ));
            }
        }
    }

    /// W108: a non-ASCII character in an argument token —
    /// either a Unicode confusable (visually resembling ASCII) or a
    /// known copy-paste artifact (smart quote, NBSP, em-dash, …).
    /// `args` / `arg_tokens` exclude the command name (the command word
    /// is not scanned).  Operates in the default **confusables** mode
    /// (→ **strict** for F5 iRules / iApps); the `common` mode — which
    /// needs Unicode general-category data Rust std lacks — is not
    /// implemented.  One diagnostic per offending character, with an
    /// ASCII-replacement fix when one is known.
    pub(in crate::analyser) fn emit_w108_non_ascii(&mut self, arg_tokens: &[tcl_lexer::Token]) {
        use super::confusables_table::{auto_fix_for, confusable_to_ascii};
        use super::state::NonAsciiMode;

        // Resolve the effective mode: an explicit `tclLsp.style.nonAscii`
        // setting, or the per-dialect default (strict for ASCII-only F5
        // dialects, confusables otherwise).
        let mode = match self.non_ascii_mode {
            NonAsciiMode::Default => {
                if self.profile.is_irules()
                    || self.profile.vendor_surface == Some(SpecProvider::Package("iapps"))
                {
                    NonAsciiMode::Strict
                } else {
                    NonAsciiMode::Confusables
                }
            }
            explicit => explicit,
        };
        if mode == NonAsciiMode::Off {
            return;
        }

        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for tok in arg_tokens {
            if seen.contains(&tok.span.start()) {
                continue;
            }
            let Some(slice) = source_slice(&self.source, tok.span) else {
                continue;
            };
            // Skip a multi-line braced body — its inner commands are
            // checked when the body is recursed into.
            if tok.kind == tcl_lexer::TokenType::Esc && slice.contains('\n') {
                continue;
            }
            // Comment regions of a braced script argument (any brace depth).
            // Comments are prose: outside strict mode only the invisible /
            // direction-altering trojan-source set can mislead review there,
            // so an em-dash or smart quote in a comment is not a finding.
            // Strict mode (ASCII-only F5 platforms) keeps flagging comment
            // bytes too — the platform constraint applies to the whole file.
            let comments = if tok.kind == tcl_lexer::TokenType::Str && mode != NonAsciiMode::Strict
            {
                // The token slice starts at the opening `{` (and stops short
                // of the closer), so lex the *content* past `content_offset`
                // and shift the regions back into slice coordinates.
                let coff = usize::from(tok.content_offset);
                slice.get(coff..).map_or_else(Vec::new, |content| {
                    comment_regions_recursive(content, self.lexer_config())
                        .into_iter()
                        .map(|r| r.start + coff..r.end + coff)
                        .collect()
                })
            } else {
                Vec::new()
            };
            let mut flagged_here = false;
            for (rel, ch) in slice.char_indices() {
                if is_standard_ascii(ch) {
                    continue;
                }
                // Bidi formatting controls belong to W305, which scans the
                // whole file — a bidi override reorders the text *around*
                // itself, so a per-argument scan is the wrong shape — and
                // reports at error severity rather than as a style warning.
                // Skipping them here is what stops one character producing two
                // codes; see `confusables_table::BIDI_CONTROLS` for the exact
                // set, and for why directional *marks* and zero-width
                // characters deliberately stay with W108.
                if super::super::confusables_table::is_bidi_control(ch) {
                    continue;
                }
                if comments.iter().any(|r| r.contains(&rel)) && !is_review_hazard_unicode(ch) {
                    continue;
                }
                let fix = auto_fix_for(ch).or_else(|| confusable_to_ascii(ch));
                let is_confusable = fix.is_some();
                // Mode-dependent filtering (strict flags everything):
                //  * confusables — confusables / auto-fix artifacts, plus
                //    the invisible / direction-altering trojan-source set
                //    (a bidi override is a review hazard wherever it sits,
                //    table membership or not);
                //  * common — those plus any non-benign character
                //    (control / format / separator / unassigned / …).
                match mode {
                    NonAsciiMode::Confusables
                        if !is_confusable && !is_review_hazard_unicode(ch) =>
                    {
                        continue;
                    }
                    NonAsciiMode::Common if !is_confusable && is_benign_unicode(ch) => continue,
                    _ => {}
                }
                flagged_here = true;
                let start = tok.span.start() + u32::try_from(rel).unwrap_or(0);
                let end = start + u32::try_from(ch.len_utf8()).unwrap_or(1);
                let span = tcl_lexer::Span::new(start, end);
                let fixes = fix
                    .map(|repl| {
                        vec![super::types::CodeFix {
                            span,
                            new_text: repl.to_string(),
                            description: "Replace with ASCII equivalent".to_string(),
                            // W108: substituting the ASCII look-alike changes the character's
                            // value. In an identifier that is the repair; inside a string literal
                            // it rewrites the program's data.
                            safety: crate::irules_checks::FixSafety::BehaviourHardening,
                        }]
                    })
                    .unwrap_or_default();
                self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W108,
    span,
    format!(
                        "Non-ASCII character U+{:04X} '{ch}' \u{2014} outside the standard ASCII \
                         printable/whitespace set",
                        ch as u32
                    ),
    super::types::Severity::Warning,
).with_fixes(fixes));
            }
            if flagged_here {
                seen.insert(tok.span.start());
            }
        }
    }

    /// W104: selected append payloads whose original executable text has a
    /// leading or trailing space. Captured payloads have no written call word.
    pub(in crate::analyser) fn emit_w104_append_list(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_append_arguments,
        };
        let Some(original) = original else { return };
        let Some(layout) = original.with_schema(source_append_arguments).flatten() else {
            return;
        };
        for argument in layout.values.clone() {
            let Some(word) = original.word(argument) else {
                continue;
            };
            if !original_word_has_pad_space(word) {
                continue;
            }
            let Some(subject) = original.subject(Kind::AppendList, Some(argument)) else {
                continue;
            };
            let fixes = Self::w104_lappend_fix(original, &layout)
                .into_iter()
                .collect();
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W104, word.span(),
                    "append with space-separated values looks like list construction. Use [lappend] instead to safely handle values containing spaces, braces, or backslashes.".to_owned(),
                    Severity::Hint,
                ).with_fixes(fixes).with_subject(subject),
            );
            return;
        }
    }

    /// Review-only proposal from exactly two complete original written operands.
    /// Captured variable/value positions and expansions cannot acquire an edit.
    fn w104_lappend_fix(
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        layout: &tcl_registry::AuthoredSourceAppendArguments,
    ) -> Option<super::types::CodeFix> {
        if layout.variable != 0
            || layout.values != (1..2)
            || original.words().arguments().len() != 2
            || original.written_index(0)? != 0
            || original.written_index(1)? != 1
        {
            return None;
        }
        let variable = original.word(0)?.try_text().ok()?;
        let value = original.word(1)?;
        if value.group().kind != tcl_lexer::WordKind::Quoted {
            return None;
        }
        let raw = value.try_text().ok()?;
        let piece = raw
            .strip_prefix('"')?
            .strip_suffix('"')?
            .strip_prefix(' ')?;
        if !is_safe_bare_word(piece) || !is_safe_bare_word(variable) {
            return None;
        }
        Some(super::types::CodeFix {
            span: original.invocation_span(),
            new_text: format!("lappend {variable} {piece}"),
            description: "Rewrite with `lappend`".to_owned(),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
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

    /// Argument indices (0-based, command-name excluded) that `cmd` reads as a
    /// plain variable *name* — the positions where a `$`-substitution is a
    /// name/value confusion (W212) and the braced `${arr}(idx)` idiom is
    /// legitimate rather than a typo (W216).
    ///
    /// Registry-driven via the `VarWrite` / `VarRead` argument roles, so every
    /// command that declares them (`set`, `incr`, `append`, `lappend`, `unset`,
    /// `info exists`, `vwait`, `catch`, `scan`, `regexp`/`regsub` captures,
    /// `dict with`/`update`, `array set`, `lassign`, …) is covered without a
    /// hand-maintained list, which drifts: separate lists for the W212 and
    /// W216 positions each end up missing what the other has.  The one
    /// structural exception is `upvar`: its frame-linking
    /// semantics are modelled by the registry's repeated layout, and only its
    /// *local*-name slots are name positions — a computed `$remote` in an
    /// other-var slot is a legitimate indirect link.
    pub(in crate::analyser) fn variable_name_positions(
        original: &super::super::diagnostic_registry::OriginalDiagnosticSource,
    ) -> Vec<(usize, tcl_registry::ArgRole)> {
        original.variable_name_arguments()
    }

    /// W212: a command argument that must be a variable
    /// *name* (`set $x 1`, `incr $x`, `info exists $x`, `upvar 1 a $b`)
    /// instead uses a `$`-substitution.  `args` / `arg_tokens` exclude
    /// the command name.  Fires when the resolved name-position argument
    /// is a `Var` token.
    ///
    /// The "Did you mean…?" suggestion is the de-sigiled name itself
    /// when that name is defined in the enclosing scope (or nothing
    /// close is); when it is *undefined* but a close in-scope variable
    /// sits within the length-scaled edit budget, the suggestion names
    /// that variable instead — `set counter 1; incr $countr` suggests
    /// 'counter', not the nonexistent 'countr'.
    pub(in crate::analyser) fn emit_w212_name_vs_value(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticSource>,
        cmd_name: &str,
        scope_path: &[usize],
    ) {
        let Some(original) = original else {
            return;
        };
        let mut scope_vars: Option<Vec<String>> = None;
        for (index, _) in Self::variable_name_positions(original) {
            let Some(word) = original.word(index) else {
                continue;
            };
            let Some(tok) = original.token(index) else {
                continue;
            };
            if tok.kind != tcl_lexer::TokenType::Var {
                continue;
            }
            let Ok(text) = word.try_text() else {
                continue;
            };
            if tcl_syntax::naming::is_braced_indirect_array_ref(text) {
                continue;
            }
            // The shared token-text owner supplies the written variable name;
            // a compound dynamic word cannot be promoted to a variable read.
            if word.tokens().len() != 1 {
                continue;
            }
            let source_map = self.cached_source_map();
            let bare = source_map.token_text(tok).to_owned();
            let bare = bare.as_str();
            let display_cmd = original.display_command(cmd_name);
            let vars = scope_vars.get_or_insert_with(|| self.scope_variable_names(scope_path));
            let suggestion = if vars.iter().any(|name| name == bare) {
                bare
            } else {
                crate::text::suggest_similar(
                    bare,
                    vars.iter().map(String::as_str),
                    1,
                    crate::text::scaled_max_distance_strict(bare),
                )
                .first()
                .copied()
                .unwrap_or(bare)
            };
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W212,
                    tok.span,
                    format!(
                        "'{display_cmd}' expects a variable name, got substitution (${bare}). \
                     Did you mean '{suggestion}'?"
                    ),
                    super::types::Severity::Warning,
                )
                .with_subject(
                    original
                        .variable_name_subject(index)
                        .expect("original written operand"),
                ),
            );
        }
    }

    fn push_w216_replacement(
        diagnostics: &mut Vec<crate::analyser::types::Diagnostic>,
        span: tcl_lexer::Span,
        mut message: String,
        corrected: Option<String>,
    ) {
        let fixes = corrected.map(|corrected| {
            use std::fmt::Write;
            let premise = if corrected.starts_with("[::set ") {
                "; this suggestion requires `::set` to retain its stock variable-reading meaning"
            } else { "" };
            let _ = write!(message, "; did you mean `{corrected}` for array element access?{premise}");
            super::types::CodeFix {
                span,
                new_text: corrected.clone(),
                description: format!("Replace with `{corrected}`{premise}"),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            }
        }).into_iter().collect();
        diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W216,
                span,
                message,
                Severity::Warning,
            )
            .with_fixes(fixes),
        );
    }

    /// **W216** — broken brace-form variants of array element access.
    ///
    /// Two related shapes both look like array-element access but parse
    /// differently than the user intends:
    ///
    /// 1. `${arr}(foo)` — the lexer ends the variable substitution at the
    ///    `}`, so this is scalar `${arr}` followed by *literal* `(foo)`; no
    ///    array access happens.
    /// 2. `${arr($foo)}` — the brace form applies no further substitution to
    ///    its content, so `$foo` inside the braces is the literal four-char
    ///    string, not the value of `foo`.
    ///
    /// In a *variable-name* position (`set` / `incr` / `append` / `lappend` /
    /// `unset` / `info exists` / `vwait`) Pattern (1) is the legitimate
    /// indirect-array-element idiom (`token` holds the array name) and must
    /// not fire — see [`tcl_syntax::naming::is_braced_indirect_array_ref`].
    pub(in crate::analyser) fn emit_w216_brace_then_paren(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
    ) {
        if cmd.texts.is_empty() {
            return;
        }
        let sm = Analyser::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let source = self.source.as_bytes();
        // Word-start offsets the command reads as a variable name; a
        // `${name}(idx)` Pattern-(1) match starting there is the indirect
        // idiom and must not fire W216.
        let mut varname_word_starts: FxHashSet<u32> = FxHashSet::default();
        if let Some(original) = self.original_diagnostic_source_for_segment(cmd) {
            for (argument, _) in Self::variable_name_positions(&original) {
                if let Some(token) = original.token(argument) {
                    varname_word_starts.insert(token.span.start());
                }
            }
        }
        for &t1 in &cmd.all_tokens {
            // Token spans are absolute into the full document; skip any token
            // whose span exceeds the current source (synthetic unit-test
            // tokens, or a span past a truncated buffer) before slicing.
            if t1.span.end() as usize > source.len() {
                continue;
            }
            if !is_brace_form_var(&sm, t1) {
                continue;
            }
            let text = sm.token_text(t1).to_string();

            // Pattern (2) — `${arr($foo)}`: the VAR token's own text contains
            // `(...)` with `$`/`[` inside.
            let (name, element) = crate::naming::split_array_name_braced(&text, true);
            if let Some(inner) = element {
                if !name.is_empty() && index_has_substitution(inner) {
                    let corrected = build_w216_replacement(name, inner, self.lexer_config());
                    // The `}` closing a `${…}` word is the owner
                    // family's call, not `span.end() + 1` — that
                    // overshoots the degenerate `${}`.
                    let span = tcl_lexer::word_span_at(&self.source, t1.span);
                    let message = format!(
                        "`${{{name}({inner})}}` does not substitute `{inner}` \
(the brace form applies no further substitution to its content)"
                    );
                    Self::push_w216_replacement(
                        &mut self.result.diagnostics,
                        span,
                        message,
                        corrected,
                    );
                }
                continue;
            }

            // Pattern (1) — `${arr}(foo)`: the VAR token ends at the last
            // char before `}`; a literal `}` follows (the exclusive span end),
            // then `(` opens a paren group.
            let close_brace = t1.span.end() as usize;
            if close_brace >= source.len() || source[close_brace] != b'}' {
                continue;
            }
            let paren_start = close_brace + 1;
            if paren_start >= source.len() || source[paren_start] != b'(' {
                continue;
            }
            let Some(paren_end) = find_matching_close_paren(source, paren_start) else {
                continue;
            };
            // Variable-name position → indirect-array idiom, suppress.
            if varname_word_starts.contains(&t1.span.start()) {
                continue;
            }
            let inner = &self.source[paren_start + 1..paren_end];
            let corrected = build_w216_replacement(&text, inner, self.lexer_config());
            let span = tcl_lexer::Span::new(
                t1.span.start(),
                u32::try_from(paren_end + 1).unwrap_or(t1.span.end()),
            );
            let message = format!(
                "`${{{text}}}({inner})` is parsed as scalar `${{{text}}}` followed by \
literal text `({inner})`"
            );
            Self::push_w216_replacement(&mut self.result.diagnostics, span, message, corrected);
        }
    }

    /// W114: a nested `[expr …]` inside an argument that is
    /// *already* an expression context (`expr` / `if` / `while` / `for`
    /// conditions) is redundant.  `diag_span` is the source span of the
    /// expression argument; we scan its source slice for the first
    /// `[`-`expr`-whitespace sequence and anchor the warning at the
    /// nested `[expr … ]`.  One warning per argument (first match only).
    ///
    /// When the outer argument is braced and the nested body is one
    /// braced group, the diagnostic carries an unwrap fix — see
    /// [`w114_unwrap_fix`] for the exact conditions.
    pub(in crate::analyser) fn emit_w114_redundant_nested_expr(
        &mut self,
        _text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        let start = diag_span.start() as usize;
        let end = diag_span.end() as usize;
        if start >= end {
            return;
        }
        let Some(slice) = Analyser::source_slice(&self.source, start, end) else {
            return;
        };
        let Some((open, close)) = first_nested_expr(slice) else {
            return;
        };
        let nested_span = tcl_lexer::Span::new(
            u32::try_from(start + open).unwrap_or(diag_span.start()),
            u32::try_from(start + close + 1).unwrap_or(diag_span.end()),
        );
        let fixes = w114_unwrap_fix(slice, open, close, nested_span)
            .into_iter()
            .collect();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W114,
                nested_span,
                "Redundant nested [expr] \u{2014} already in expression context".to_string(),
                super::types::Severity::Warning,
            )
            .with_fixes(fixes),
        );
    }

    /// **W110.** Emit "use `eq`/`ne` instead of `==`/`!=` for
    /// string comparison" hints on the EXPR-role argument of
    /// commands like `if`, `while`, `for`, `expr`.
    ///
    /// Fires only where the rewrite is proven to keep the result: in a
    /// braced argument, a `==` / `!=` one of whose operands is a fixed string
    /// that is not a number in any release, so Tcl already compares the two as
    /// strings ([`eq_ne_compares_as_strings`]). `$x == "42"` is a numeric
    /// compare when `x` is `42.0`, and `"$x" == 1` when `x` is `1.0`, so
    /// neither draws the hint; `$x == "foo"` does.
    ///
    /// `expr_text` is the post-substitution body of the EXPR-role
    /// argument (already brace-stripped) — the caller is
    /// responsible for joining multi-arg `expr` invocations with
    /// spaces before calling.  `diag_span` is the fallback span (the
    /// source range of the argument token, or the full token range for
    /// `expr`); the diagnostic anchors on the offending *operator*
    /// itself when `anchor` lets its offset map back to real source
    /// bytes.  The code fix always spans the whole argument
    /// (`diag_span`) — the rewrite replaces the full expression text.
    pub(in crate::analyser) fn emit_w110_string_eq_ne(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
        anchor: &W110Anchor<'_>,
    ) {
        // Quick bail-out: no equality operator at all.
        if !expr_text.contains("==") && !expr_text.contains("!=") {
            return;
        }
        let trimmed = expr_text.trim();
        let Some(context) = self.original_expression_parser_context() else {
            return;
        };
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(trimmed, &context);
        // ``ExprNode::Raw`` means the expression was unparseable.
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        // Only a braced argument reaches `expr` as written; any other word is
        // substituted first, so its operands are not the ones parsed here.
        let braced = matches!(anchor, W110Anchor::ArgToken(tok)
            if self.source.as_bytes().get(tok.span.start() as usize) == Some(&b'{'));
        if !braced {
            return;
        }
        let matched_ops = find_string_eq_ne_ops(&parsed, trimmed);
        if matched_ops.is_empty() {
            return;
        }
        let (first_op, first_off) = matched_ops[0];
        let (op_text, replacement) = match first_op {
            BinOp::Eq => ("==", "eq"),
            BinOp::Ne => ("!=", "ne"),
            _ => unreachable!("find_string_eq_ne_ops only returns Eq/Ne"),
        };
        // Anchor the diagnostic on the matched operator when its offset in
        // the parsed text maps back to verbatim source bytes; the argument
        // span is the fallback.  Offsets are relative to the *trimmed*
        // text, so shift by the leading whitespace the trim removed.
        let trim_off = expr_text.len() - expr_text.trim_start().len();
        let span = first_off
            .and_then(|off| self.w110_operator_span(anchor, trim_off + off as usize, op_text))
            .unwrap_or(diag_span);
        // The fix rewrites exactly the proven operators, in place, and only
        // when every one of them maps back to its source bytes.
        let mut fixes = Vec::new();
        if let Some(fix) = self.w110_operator_fix(anchor, trim_off, &matched_ops, replacement) {
            fixes.push(fix);
        }
        let message = format!(
            "Use '{replacement}' instead of '{op_text}' for string \
comparison in expressions to avoid ambiguous \
numeric/string coercion."
        );
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::W110, span, message, Severity::Hint)
                .with_fixes(fixes),
        );
    }

    /// One edit over the proven operators: each `==` / `!=` becomes `eq` /
    /// `ne`, spaced from its neighbours, and every other byte stays as
    /// written. Each rewritten operator compares as strings already
    /// ([`eq_ne_compares_as_strings`]), so the rewrite keeps the result.
    fn w110_operator_fix(
        &self,
        anchor: &W110Anchor<'_>,
        trim_off: usize,
        ops: &[(BinOp, Option<u32>)],
        replacement: &str,
    ) -> Option<super::types::CodeFix> {
        let mut spans = Vec::with_capacity(ops.len());
        for (op, off) in ops {
            let (from, to) = match op {
                BinOp::Eq => ("==", "eq"),
                BinOp::Ne => ("!=", "ne"),
                _ => return None,
            };
            let span = self.w110_operator_span(anchor, trim_off + (*off)? as usize, from)?;
            spans.push((span.start() as usize, span.end() as usize, to));
        }
        spans.sort_unstable();
        let (first, last) = (spans.first()?.0, spans.last()?.1);
        let bytes = self.source.as_bytes();
        let spaced = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_whitespace);
        let mut new_text = String::new();
        let mut at = first;
        for (start, end, to) in spans {
            new_text.push_str(self.source.get(at..start)?);
            if start > 0 && !spaced(start - 1) {
                new_text.push(' ');
            }
            new_text.push_str(to);
            if !spaced(end) {
                new_text.push(' ');
            }
            at = end;
        }
        Some(super::types::CodeFix {
            span: tcl_lexer::Span::new(u32::try_from(first).ok()?, u32::try_from(last).ok()?),
            new_text,
            description: format!("Use '{replacement}' for string comparison"),
            safety: crate::irules_checks::FixSafety::SemanticsEquivalent,
        })
    }

    /// Map a W110 operator offset (within the emitter's `expr_text`) to
    /// its absolute source span, verifying the source bytes actually spell
    /// `op_text` there (reconstructed argument texts fail the check →
    /// `None`, and the caller falls back to the argument span).
    fn w110_operator_span(
        &self,
        anchor: &W110Anchor<'_>,
        off: usize,
        op_text: &str,
    ) -> Option<tcl_lexer::Span> {
        let candidate = match anchor {
            // Single argument word: content starts `content_offset` bytes
            // into the token span.
            W110Anchor::ArgToken(tok) => {
                tok.span.start() as usize + usize::from(tok.content_offset) + off
            }
            // `expr $a == "x"` multi-word form: `expr_text` is
            // `args.join(" ")`, so locate which word the offset falls in
            // and anchor at that word's own token.
            W110Anchor::JoinedWords { args, tokens } => {
                let mut cum = 0usize;
                let mut found = None;
                for (i, arg) in args.iter().enumerate() {
                    if off < cum + arg.len() {
                        found = Some((i, off - cum));
                        break;
                    }
                    cum += arg.len() + 1;
                }
                let (word_idx, in_word) = found?;
                let tok = tokens.get(word_idx)?;
                tok.span.start() as usize + usize::from(tok.content_offset) + in_word
            }
        };
        // Byte-verify: the span is only trustworthy when the source spells
        // the operator at the candidate position.
        if self.source.get(candidate..candidate + op_text.len())? != op_text {
            return None;
        }
        Some(tcl_lexer::Span::new(
            u32::try_from(candidate).ok()?,
            u32::try_from(candidate + op_text.len()).ok()?,
        ))
    }
}

/// How [`Analyser::emit_w110_string_eq_ne`] maps an operator offset within
/// its `expr_text` back to source bytes.
pub(in crate::analyser) enum W110Anchor<'a> {
    /// The single EXPR-role argument's own token — `expr_text` is that
    /// word's (possibly brace-stripped) content.
    ArgToken(tcl_lexer::Token),
    /// The joined multi-word `expr $a == "x"` form — `expr_text` is
    /// `args.join(" ")` over these words/tokens.
    JoinedWords {
        /// The argument word texts that were joined.
        args: &'a [String],
        /// The words' representative tokens, same order.
        tokens: &'a [tcl_lexer::Token],
    },
}

/// True when `tok` is a `${name}` (brace-form) VAR token.  Bare `$name` spans
/// `name.len() + 1` (one `$`); brace `${name}` spans more (`${` + name).
/// The [`tcl_lexer::Span`] end is exclusive, so the span length is
/// `end - start`.
fn is_brace_form_var(sm: &SourceMap<'_>, tok: tcl_lexer::Token) -> bool {
    if tok.kind != tcl_lexer::TokenType::Var {
        return false;
    }
    let span_len = (tok.span.end() - tok.span.start()) as usize;
    span_len > sm.token_text(tok).len() + 1
}

/// Return `true` if `inner` contains a `$` or `[` the user likely expects to
/// substitute — the trigger for the `${arr($foo)}` Pattern-(2) variant of
/// W216.  Skips backslash escapes.
fn index_has_substitution(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'$' | b'[' => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

/// Propose array-index substitution while preserving the literal source root.
/// The fallback invokes an unwritten `::set`; its stock meaning is a separate
/// RequiresReview premise, never an original command selection proof.
fn build_w216_replacement(
    name: &str,
    inner: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    // naming.diagnostics.original-w216-literal-source-replacement
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-w216-literal-source-replacement.md
    if name.is_empty() || name.contains('(') {
        return None;
    }
    let candidate = format!("$a({inner})");
    let reference = tcl_lexer::scan_var_ref(candidate.as_bytes(), 0, config).ok()??;
    if reference.next != candidate.len() || reference.index.is_none() {
        return None;
    }
    if tcl_syntax::naming::is_bare_var_name(name) {
        Some(format!("${name}({inner})"))
    } else {
        let root = tcl_syntax::backslash::literal_quoted_source_fragment(name, config.escapes)?;
        let index = tcl_syntax::backslash::substituting_quoted_source_fragment(inner, config)?;
        Some(format!("[::set \"{root}({index})\"]"))
    }
}

/// Find the offset of the `)` matching the `(` at `paren_start`.  Skips
/// balanced `{...}`, double-quoted strings, and backslash escapes.  Returns
/// `None` on malformed input.
fn find_matching_close_paren(source: &[u8], paren_start: usize) -> Option<usize> {
    let n = source.len();
    let mut depth = 1i32;
    let mut j = paren_start + 1;
    let mut in_quote = false;
    while j < n && depth > 0 {
        let c = source[j];
        if in_quote {
            if c == b'\\' && j + 1 < n {
                j += 2;
                continue;
            }
            if c == b'"' {
                in_quote = false;
            }
            j += 1;
            continue;
        }
        match c {
            b'"' => {
                in_quote = true;
                j += 1;
                continue;
            }
            b'{' => {
                let mut bd = 1i32;
                j += 1;
                while j < n && bd > 0 {
                    if source[j] == b'\\' && j + 1 < n {
                        j += 2;
                        continue;
                    }
                    match source[j] {
                        b'{' => bd += 1,
                        b'}' => bd -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                continue;
            }
            b'\\' if j + 1 < n => {
                j += 2;
                continue;
            }
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    if depth != 0 { None } else { Some(j - 1) }
}

/// True when `ch` is a standard ASCII character W108 leaves alone: tab
/// / LF / CR, or printable ASCII `0x20`-`0x7e`.
fn is_standard_ascii(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | ' '..='~')
}

/// W108 "common" mode benign-Unicode test. A character is
/// *intentional* (not flagged) when its Unicode general category is a
/// Letter, Number, Mark, Symbol, or Punctuation (any script). Control,
/// True when `ch` is invisible or direction-altering — the trojan-source
/// set that can make reviewed text lie about the code it sits next to:
/// bidi embedding / override / isolate controls and other format
/// characters (`Cf`), non-ASCII control characters (`Cc`), and the Unicode
/// line / paragraph separators (`Zl` / `Zp`).  These stay flagged inside
/// comments (where ordinary non-ASCII prose is fine) and bypass the
/// confusables-table gate in code.
pub(super) fn is_review_hazard_unicode(ch: char) -> bool {
    use unicode_general_category::{GeneralCategory as G, get_general_category};
    matches!(
        get_general_category(ch),
        G::Format | G::Control | G::LineSeparator | G::ParagraphSeparator
    )
}

/// Byte ranges of `slice` (a braced script argument's content) covered by
/// comments, at any brace depth: the comment tokens of the slice's own
/// lex, plus — recursively — those of every nested braced word, offset
/// into this slice's coordinates.  A slice that fails to lex contributes
/// no regions (conservative: everything stays scanned).  A braced plain
/// string whose content merely *looks* like a comment under-flags — the
/// acceptable cost of not knowing which braced words are scripts.
fn comment_regions_recursive(
    slice: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<std::ops::Range<usize>> {
    fn collect(
        slice: &str,
        base: usize,
        config: tcl_lexer::LexerConfig,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        if !slice.contains('#') {
            return;
        }
        let Ok(tokens) = tcl_lexer::Lexer::with_config(slice, config).tokenise_all() else {
            return;
        };
        for tok in &tokens {
            let start = tok.span.start() as usize;
            let end = tok.span.end() as usize;
            match tok.kind {
                tcl_lexer::TokenType::Comment => out.push(base + start..base + end),
                tcl_lexer::TokenType::Str => {
                    // Recurse on the brace *content* — the token span covers
                    // the opener, which would unbalance a nested lex.
                    let coff = usize::from(tok.content_offset);
                    if let Some(inner) = slice.get(start + coff..end) {
                        collect(inner, base + start + coff, config, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(slice, 0, config, &mut out);
    out
}

/// format, separator, surrogate, private-use, and unassigned characters
/// are *not* benign (they almost always indicate encoding/copy-paste
/// issues) and are flagged.
pub(super) fn is_benign_unicode(ch: char) -> bool {
    use unicode_general_category::{GeneralCategory as G, get_general_category};
    matches!(
        get_general_category(ch),
        // Letters (L*)
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
            // Numbers (N*)
            | G::DecimalNumber
            | G::LetterNumber
            | G::OtherNumber
            // Marks (M*)
            | G::NonspacingMark
            | G::SpacingMark
            | G::EnclosingMark
            // Symbols (S*)
            | G::MathSymbol
            | G::CurrencySymbol
            | G::ModifierSymbol
            | G::OtherSymbol
            // Punctuation (P*)
            | G::ConnectorPunctuation
            | G::DashPunctuation
            | G::OpenPunctuation
            | G::ClosePunctuation
            | G::InitialPunctuation
            | G::FinalPunctuation
            | G::OtherPunctuation
    )
}

/// Mask-octet values that can appear in a contiguous subnet mask.
const VALID_MASK_OCTETS: &[u32] = &[0, 128, 192, 224, 240, 248, 252, 254, 255];

/// True when the four octets form a valid contiguous subnet mask
/// (all-1s then all-0s).
pub(super) fn is_valid_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    if val == 0 {
        return true;
    }
    let inverted = val ^ 0xFFFF_FFFF;
    (inverted & inverted.wrapping_add(1)) == 0
}

/// Heuristic: the dotted-quad plausibly *intends* to be a mask.
pub(super) fn looks_like_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    if a == 255 && !(b == 255 && c == 255 && d == 255) {
        return true;
    }
    a >= 128 && [a, b, c, d].iter().all(|o| VALID_MASK_OCTETS.contains(o))
}

/// Suggest the nearest valid contiguous mask, or `None`.
pub(super) fn nearest_valid_mask(a: u32, b: u32, c: u32, d: u32) -> Option<String> {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    let mut leading = 0u32;
    for bit in (0..32).rev() {
        if val & (1 << bit) != 0 {
            leading += 1;
        } else {
            break;
        }
    }
    if leading == 0 || leading == 32 {
        return None;
    }
    let candidate = 0xFFFF_FFFFu32 << (32 - leading);
    Some(format!(
        "{}.{}.{}.{}",
        (candidate >> 24) & 0xFF,
        (candidate >> 16) & 0xFF,
        (candidate >> 8) & 0xFF,
        candidate & 0xFF
    ))
}

/// True when `text` is a simple numeric or boolean literal that needs
/// no bracing.
pub(super) fn is_safe_literal(text: &str) -> bool {
    let t = text.trim();
    if t.parse::<f64>().is_ok() {
        return true;
    }
    matches!(
        t.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    )
}

/// True when an expr string is substitution-free numeric / boolean /
/// operator text (safe to leave unbraced).
#[cfg(test)]
pub(super) fn is_safe_literal_expr(
    text: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> bool {
    is_safe_literal_expr_in_context(
        text,
        &tcl_syntax::expr::parser::ExprParseContext::for_profile(profile),
    )
}

fn is_safe_literal_expr_in_context(
    text: &str,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> bool {
    use tcl_lexer::ExprTokenType as T;
    if is_safe_literal(text) {
        return true;
    }
    if text.contains('$') || text.contains('[') {
        return false;
    }
    let (tokens, unknown) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        text,
        &context.lexer_grammar,
        context.expr_grammar_base,
        context.f5_word_grammar,
    );
    if unknown {
        return false;
    }
    if tokens.is_empty() {
        return false;
    }
    tokens.iter().all(|tok| {
        matches!(
            tok.kind,
            T::Number
                | T::Bool
                | T::Operator
                | T::ParenOpen
                | T::ParenClose
                | T::Whitespace
                | T::TernaryQ
                | T::TernaryC
                | T::Comma
        )
    })
}

/// The *local*-name argument indices (into `args`, command-name excluded) of an
/// `upvar ?level? otherVar localVar ?otherVar localVar …?` call — every other
/// argument after the optional leading level word.
///
/// `upvar` is resolved structurally rather than through the registry's
/// `VarWrite` role: its frame-linking semantics are modelled by the dedicated
/// var-escape machinery, and only the *local* names are strict name positions
/// (the paired *other* names may legitimately be a computed `$remote`). See
/// `Analyser::variable_name_positions`.
/// Find the first `[`-`expr`-whitespace sequence in `slice` (the
/// `_NESTED_EXPR_RE = \[\s*expr\s` pattern) and return
/// `(open_bracket_index, matching_close_bracket_index)`.  The close is
/// located by a depth scan; an unmatched `[` falls back to the last
/// byte.  Returns `None` when no nested `[expr ` is present.
pub(super) fn first_nested_expr(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let len = bytes.len();
    // Bracket-substitution nesting depth.  Only a *top-level* (depth-0)
    // `[expr …]` command substitution is "already in expression context" and
    // therefore redundant.  An `[expr …]` nested inside another command
    // substitution's arguments — e.g. `if {[myCmd [expr {1+1}]]}` — is a fresh
    // command-argument context, not an expression context, so it must NOT be
    // flagged.
    let mut depth: i32 = 0;
    let mut open = 0;
    while open < len {
        match bytes[open] {
            b'[' => {
                if depth == 0 {
                    // `\s*`
                    let mut after_ws = open + 1;
                    while after_ws < len && bytes[after_ws].is_ascii_whitespace() {
                        after_ws += 1;
                    }
                    // `expr` followed by a whitespace byte.
                    let kw_end = after_ws + 4;
                    if kw_end < len
                        && &bytes[after_ws..kw_end] == b"expr"
                        && bytes[kw_end].is_ascii_whitespace()
                    {
                        // Depth-scan for the matching `]` (the open `[` is
                        // already counted).
                        let mut d = 1;
                        let mut scan = open + 1;
                        while scan < len && d > 0 {
                            match bytes[scan] {
                                b'[' => d += 1,
                                b']' => d -= 1,
                                _ => {}
                            }
                            scan += 1;
                        }
                        let close = if d == 0 { scan - 1 } else { len - 1 };
                        return Some((open, close));
                    }
                }
                depth += 1;
            }
            b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        open += 1;
    }
    None
}

/// The W114 unwrap fix: replace the nested `[expr {INNER}]` (at
/// `open..=close` within `outer`, absolute span `span`) with
/// `(INNER)` — or bare `INNER` when it is a single atom.  `None`
/// unless every condition for a purely textual inline holds:
///
/// * the outer argument is braced (`outer` starts with `{`), so no
///   Tcl-level substitution reruns over the inlined text;
/// * the bracket scan really matched (`outer[close]` is `]`);
/// * the outer expression carries no string-comparison operator
///   (`eq`/`ne`/`in`/`ni`): a nested `expr` normalises a numeric
///   result (`[expr {$x}]` yields `7` for `x == "007"`), so unwrapping
///   could flip a string comparison's verdict;
/// * the nested body is exactly one braced group — an unbraced body
///   would be re-substituted when inlined.
fn w114_unwrap_fix(
    outer: &str,
    open: usize,
    close: usize,
    span: tcl_lexer::Span,
) -> Option<super::types::CodeFix> {
    if !outer.starts_with('{') || outer.as_bytes().get(close) != Some(&b']') {
        return None;
    }
    if contains_string_comparison_word(outer) {
        return None;
    }
    // Step over `[`, `\s*`, `expr`, and the following whitespace run to
    // reach the body region (mirrors `first_nested_expr`'s match).
    let bytes = outer.as_bytes();
    let mut body_start = open + 1;
    while body_start < close && bytes[body_start].is_ascii_whitespace() {
        body_start += 1;
    }
    body_start = body_start.checked_add(4)?; // the `expr` keyword
    while body_start < close && bytes[body_start].is_ascii_whitespace() {
        body_start += 1;
    }
    let body = outer.get(body_start..close)?.trim();
    let inner = single_braced_group(body)?.trim();
    if inner.is_empty() {
        return None;
    }
    let new_text = if is_expr_atom(inner) {
        inner.to_string()
    } else {
        format!("({inner})")
    };
    Some(super::types::CodeFix {
        span,
        new_text,
        description: "Unwrap the nested `expr`".to_string(),
        // W114: unwrapping the nested `expr` removes a
        // stringify/re-parse round, which can change the numeric formatting
        // of a floating-point intermediate.
        safety: crate::irules_checks::FixSafety::RequiresReview,
    })
}

/// The content of `body` when it is exactly one `{…}` group — braces
/// balanced, depth first returning to zero at the final byte.  `None`
/// otherwise (several groups, unbalanced or quote/escape-skewed braces,
/// trailing text).
fn single_braced_group(body: &str) -> Option<&str> {
    let bytes = body.as_bytes();
    if body.len() < 2 || bytes[0] != b'{' || bytes[body.len() - 1] != b'}' {
        return None;
    }
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 || (depth == 0 && i != body.len() - 1) {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then(|| &body[1..body.len() - 1])
}

/// The word-form `expr` operators whose verdict a nested `expr`'s numeric
/// normalisation can influence — string equality (`eq`/`ne`), string
/// ordering (`lt`/`le`/`gt`/`ge`, TIP 461), and list membership (`in`/`ni`).
/// Derived from `tcl_syntax::expr::operators`: every `BinOp` whose mathop
/// shape is a string-only `BoolChain`/`NeBinary`, or `Membership`.  A
/// hand-typed list misses `lt`/`le`/`gt`/`ge`, which carry exactly the same
/// numeric-normalisation risk as `eq`/`ne` (`{[expr {$x}] lt "007"}` can have
/// its verdict silently flipped by W114's unwrap fix the same way `==`'s can).
fn string_comparison_op_spellings() -> &'static [&'static str] {
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, OperatorShape};
    static OPS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                let is_string_family = matches!(
                    spec.mathop_shape,
                    Some(
                        OperatorShape::BoolChain { string_only: true }
                            | OperatorShape::NeBinary { string_only: true }
                            | OperatorShape::Membership { .. }
                    )
                );
                is_string_family.then_some(spec.spelling)
            })
            .collect()
    })
}

/// True when `text` contains a standalone word from
/// [`string_comparison_op_spellings`] — the expr string-comparison
/// operators whose verdict a nested `expr`'s numeric normalisation can
/// influence.  Deliberately scans the whole outer expression (a coarse,
/// conservative over-match).
fn contains_string_comparison_word(text: &str) -> bool {
    let bytes = text.as_bytes();
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$';
    for op in string_comparison_op_spellings() {
        let mut from = 0;
        while let Some(pos) = text.get(from..).and_then(|t| t.find(op)) {
            let at = from + pos;
            let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
            let after = at + op.len();
            let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
            from = at + 1;
        }
    }
    false
}

/// True when `inner` is a single expr atom — a bare
/// alphanumeric/`.`/`_` literal (`42`, `3.14`, `0x1f`, `true`) or a
/// plain `$name` / `${name}` variable reference — so the `(…)`
/// wrapping of the W114 unwrap fix can be dropped without changing
/// how the surrounding expression parses.
fn is_expr_atom(inner: &str) -> bool {
    let ident = |name: &str| {
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
    };
    if let Some(name) = inner.strip_prefix('$') {
        return match name.strip_prefix('{').and_then(|n| n.strip_suffix('}')) {
            Some(braced) => ident(braced),
            None => ident(name),
        };
    }
    !inner.is_empty()
        && inner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_'))
}

/// True when `text` can be pasted verbatim as one bare Tcl word — no
/// whitespace and none of the word/list metacharacters that would
/// change tokenisation (`{`/`}`/`"`/`[`/`]`/`\`/`;`).
fn is_safe_bare_word(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '{' | '}' | '"' | '[' | ']' | '\\' | ';'))
}

/// Walk `node` and collect every `==`/`!=` operator that compares as strings
/// whatever its other operand holds ([`eq_ne_compares_as_strings`]), paired with
/// the operator's byte offset within `text` (the parsed expression
/// source) when it can be located — `None` when an operand extent is
/// unavailable (a `Raw` child) or the operator text is not found in the
/// gap between the operands.
///
/// Comparisons between
/// two variables (`$x == $y`) are intentionally *not* collected —
/// the variables may hold integer values, making `==` correct.
fn find_string_eq_ne_ops(node: &ExprNode, text: &str) -> Vec<(BinOp, Option<u32>)> {
    let mut found = Vec::new();
    walk_string_eq_ne(node, text, &mut found, 0);
    found
}

fn walk_string_eq_ne(
    node: &ExprNode,
    text: &str,
    found: &mut Vec<(BinOp, Option<u32>)>,
    depth: u32,
) {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the `==`/`!=`-against-string findings gathered so far is
    // the safe fallback (occurrences buried deeper than the cap go
    // unreported; never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_string_eq_ne(left, text, found, depth + 1);
            walk_string_eq_ne(right, text, found, depth + 1);
            if matches!(op, BinOp::Eq | BinOp::Ne) && eq_ne_compares_as_strings(left, right) {
                found.push((*op, op_offset_between(text, left, right, *op)));
            }
        }
        ExprNode::Unary { operand, .. } => walk_string_eq_ne(operand, text, found, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_string_eq_ne(condition, text, found, depth + 1);
            walk_string_eq_ne(true_branch, text, found, depth + 1);
            walk_string_eq_ne(false_branch, text, found, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_string_eq_ne(arg, text, found, depth + 1);
            }
        }
        _ => {}
    }
}

/// The `(start, end)` extent of `node` within its parsed text, from its
/// leaves' offsets (`end` inclusive — the expr lexer's convention).
/// `None` when a `Raw` child makes the extent unknowable.
fn node_extent(node: &ExprNode) -> Option<(u32, u32)> {
    // Entry point: the top of an expression tree is nesting depth 0 (the
    // recursion cap lives in [`node_extent_at`]).
    node_extent_at(node, 0)
}

fn node_extent_at(node: &ExprNode, depth: u32) -> Option<(u32, u32)> {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, report an unknowable extent
    // (`None`) — the same conservative answer this function already returns
    // for a `Raw` child, so callers fall back to a coarser span rather than
    // crashing.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return None;
    }
    match node {
        ExprNode::Literal { start, end, .. }
        | ExprNode::String { start, end, .. }
        | ExprNode::Var { start, end, .. }
        | ExprNode::Command { start, end, .. }
        | ExprNode::Call { start, end, .. } => Some((*start, *end)),
        ExprNode::Binary { left, right, .. } => {
            let (ls, _) = node_extent_at(left, depth + 1)?;
            let (_, re) = node_extent_at(right, depth + 1)?;
            Some((ls, re))
        }
        ExprNode::Unary { operand, .. } => node_extent_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            false_branch,
            ..
        } => {
            let (cs, _) = node_extent_at(condition, depth + 1)?;
            let (_, fe) = node_extent_at(false_branch, depth + 1)?;
            Some((cs, fe))
        }
        // `CompiledWord` carries no source span either: it is a *word* the IR
        // reduced to its value, not a slice of the expression text this
        // extent indexes into.
        ExprNode::Raw { .. } | ExprNode::CompiledWord { .. } => None,
    }
}

/// Byte offset of `op`'s source text within `text`, located in the gap
/// between the left operand's (inclusive) end and the right operand's
/// start.  This pins the *matched* operator, not merely the first
/// occurrence — `$a == $b && $c == "x"` anchors the second `==`.
fn op_offset_between(text: &str, left: &ExprNode, right: &ExprNode, op: BinOp) -> Option<u32> {
    let (_, left_end) = node_extent(left)?;
    let (right_start, _) = node_extent(right)?;
    let gap_start = (left_end as usize) + 1;
    let gap = text.get(gap_start..right_start as usize)?;
    let rel = gap.find(op.as_str())?;
    u32::try_from(gap_start + rel).ok()
}

#[cfg(test)]
mod issue996_tests {
    use super::*;
    use crate::expr_ast::UnaryOp;

    /// Depth coverage: `walk_string_eq_ne` and `node_extent` each recurse once per `ExprNode` level
    /// (`walk_string_eq_ne` also drives `node_extent` via
    /// `op_offset_between`), so without a depth cap they overflow the native
    /// stack (SIGABRT) in the low thousands of levels on a 2 MiB thread.  A
    /// tree built directly is unbounded (the Pratt parser caps its own output
    /// at 256); 3000 is past that crash range and past
    /// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that each returns at all.
    #[test]
    fn deeply_nested_eq_ne_walks_survive() {
        let mut node = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = find_string_eq_ne_ops(&node, "");
        let _ = node_extent(&node);
    }
}

#[cfg(test)]
mod original_expression_grammar_tests {
    use super::*;

    #[test]
    fn original_string_comparison_advice_uses_retained_variable_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source parser and warning/edit geometry only; no runtime equivalence.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = r#"expr {${a{b}c} == "needle"}"#;
        for (braced_var, expected) in [
            (tcl_dialect::BracedVarStyle::FirstClose, 0),
            (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
        ] {
            let config = tcl_lexer::LexerConfig {
                braced_var,
                ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
            };
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::clone(&context),
                config,
            );
            let mut analyser = Analyser::new().with_resolved_input(input);
            let result = analyser.analyse(source, "presentation-only");
            let hints: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W110)
                .collect();
            assert_eq!(
                hints.len(),
                expected,
                "{braced_var:?}: {:?}",
                result.diagnostics
            );
            if let Some(hint) = hints.first() {
                assert_eq!(source.get(hint.span.as_range()), Some("=="));
                assert_eq!(hint.fixes.len(), 1);
                assert_eq!(hint.fixes[0].new_text, "eq");
            }
            assert_eq!(
                analyser
                    .original_expression_parser_context()
                    .unwrap()
                    .lexer_grammar
                    .braced_var,
                braced_var
            );
            analyser.result.resolved_input = None;
            assert!(analyser.original_expression_parser_context().is_none());
            analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment("tcl8.4")
                    .default_context_registry(),
                config,
            ));
            assert!(analyser.original_expression_parser_context().is_none());
        }
    }
}
