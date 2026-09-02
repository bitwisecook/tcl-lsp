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

use rustc_hash::FxHashSet;
use tcl_core_types::DiagCode;
use tcl_lexer::SourceMap;

use super::helpers::{find_dotted_quads, has_substitution, is_braced_word, source_slice};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use tcl_dialect::model::SpecProvider;

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

    /// W100: an expression argument (`expr` / `if` / `while`
    /// / `for` conditions) that is not braced suffers double
    /// substitution and defeats byte-compilation.  Skips a braced
    /// (`{…}`) argument and a substitution-free numeric/boolean literal;
    /// otherwise emits W100 (ERROR when the text carries a `$`/`[`
    /// substitution, else WARNING) with a brace-wrapping fix.
    /// `args` / `arg_tokens` exclude the command name.
    pub(in crate::analyser) fn emit_w100_unbraced_expr(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut indices = registry.arg_indices_for_role(
            cmd_name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Expr,
        );
        if indices.is_empty() {
            return;
        }
        indices.sort_unstable();

        // Whether this command concatenates its whole argument tail into
        // one expression (the registry's `EXPR_CONCATENATES_ARGS` trait —
        // `expr`), so W100 anchors at the full tail span rather than one
        // argument.
        let is_expr = registry.get(cmd_name).is_some_and(|s| {
            s.traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        });
        let dialect = self.dialect();
        // The whole-`expr` argument span (used when the command is
        // `expr`, whose expression is the remaining words).
        let expr_full_span = (!arg_tokens.is_empty()).then(|| {
            tcl_lexer::Span::new(
                arg_tokens[0].span.start(),
                arg_tokens[arg_tokens.len() - 1].span.end(),
            )
        });
        let any_sub_token = arg_tokens.iter().any(|t| {
            matches!(
                t.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            )
        });

        let mut pending: Vec<(tcl_lexer::Span, String, bool)> = Vec::new();
        for idx in indices {
            let (Some(tok), Some(arg_text)) = (arg_tokens.get(idx), args.get(idx)) else {
                continue;
            };
            // A braced word (`{…}`, i.e. a `Str` token) is already safe.
            if tok.kind == tcl_lexer::TokenType::Str {
                continue;
            }
            // Resolve the diagnostic span + text: for `expr` the whole
            // remaining-argument span; otherwise the single argument's
            // source slice (preserving `$var` substitutions).
            let (span, text) = if is_expr {
                let sp = expr_full_span.unwrap_or(tok.span);
                (
                    sp,
                    source_slice(&self.source, sp).unwrap_or_else(|| args.join(" ")),
                )
            } else {
                (
                    tok.span,
                    source_slice(&self.source, tok.span).unwrap_or_else(|| arg_text.clone()),
                )
            };
            let stripped = text.trim();
            let safe = if is_expr {
                is_safe_literal_expr(stripped, dialect)
            } else {
                is_safe_literal(stripped)
            };
            if safe {
                continue;
            }
            let has_sub = text.contains('$')
                || text.contains('[')
                || if is_expr {
                    any_sub_token
                } else {
                    matches!(
                        tok.kind,
                        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                    )
                };
            pending.push((span, text, has_sub));
        }

        for (span, text, has_sub) in pending {
            self.push_w100(cmd_name, is_expr, span, &text, has_sub);
        }
    }

    /// Push one W100 diagnostic for an unbraced expression argument.
    fn push_w100(
        &mut self,
        cmd_name: &str,
        is_expr: bool,
        span: tcl_lexer::Span,
        text: &str,
        has_sub: bool,
    ) {
        // Deliberate severity escalation (documented in the DiagCode
        // catalogue): an unbraced expression that provably substitutes is
        // evaluated twice at runtime — a correctness/injection risk, not
        // just a byte-compilation loss.
        let severity = if has_sub {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        let message = if is_expr {
            "Expression is not braced: may cause double substitution and prevents \
             byte-compilation. Use expr {...} instead."
                .to_string()
        } else {
            format!(
                "Expression argument to '{cmd_name}' is not braced: may cause double \
                 substitution. Use braces: {{{text}}}"
            )
        };
        // Brace-wrapping fix; for a quoted `expr "…"` drop the quotes.
        let fix_inner = if is_expr {
            let s = text.trim();
            if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                &s[1..s.len() - 1]
            } else {
                text
            }
        } else {
            text
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::W100, span, message, severity)
                .with_fixes(vec![super::types::CodeFix {
                    span,
                    new_text: format!("{{{fix_inner}}}"),
                    description: "Wrap expression in braces".to_string(),
                    // Per-instance: `expr 1 + 2` → `expr {1 + 2}` reaches `expr`
                    // with the same string, but `expr $a + $b` → `expr {$a + $b}`
                    // stops `$a` being substituted before `expr` parses it, which
                    // is a real behaviour change when `$a` holds expression text
                    // (issue #1195).
                    safety: super::helpers::brace_wrap_fix_safety(text, has_sub),
                }]),
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
        let opt_start = if cmd_name == "fconfigure" {
            1
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

    /// W200: a `u` / `s` modifier on a `binary format` / `binary
    /// scan` integer specifier requires Tcl 8.5+ (TIP 275). Sites are
    /// buffered and decided post-walk against the effective Tcl version
    /// (§6 argument-DSL rung) — the old hardcoded dialect list wrongly
    /// included f5-iapps, whose host embeds a real Tcl 8.5.13 where the
    /// modifiers work.
    pub(in crate::analyser) fn emit_w200_binary_format_modifiers(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "binary" || args.is_empty() {
            return;
        }
        let fmt_idx = match args[0].as_str() {
            "format" if args.len() >= 2 => 1,
            "scan" if args.len() >= 3 => 2,
            _ => return,
        };
        let Some(fmt_tok) = arg_tokens.get(fmt_idx) else {
            return;
        };
        let fmt = args[fmt_idx].as_bytes();
        let mut i = 0;
        while i < fmt.len() {
            if fmt[i].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                i += 1;
            }
            if i >= fmt.len() {
                break;
            }
            let spec = fmt[i];
            i += 1;
            if BINARY_INT_SPECIFIERS.contains(&spec)
                && i < fmt.len()
                && (fmt[i] == b'u' || fmt[i] == b's')
            {
                let modifier = fmt[i] as char;
                self.dsl_gate_sites.push(super::version_gate::DslGateSite {
                    span: fmt_tok.span,
                    code: DiagCode::W200,
                    what: format!(
                        "signed/unsigned modifier '{modifier}' on binary format specifier"
                    ),
                    min: tcl_dialect::TclVersion::V8_5,
                });
                i += 1;
            }
            if i < fmt.len() && fmt[i] == b'*' {
                i += 1;
            }
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
    /// needs Unicode general-category data Rust std lacks — is not yet
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
                // characters deliberately stay with W108. Issue #1326.
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

    /// W104: `append` used with a space-padded value looks
    /// like list construction — fragile if the data contains special
    /// characters.  Fires once (HINT) on the first value argument that
    /// starts or ends with a space.
    ///
    /// The two-word leading-space shape — `append var " $item"`, one
    /// quoted value holding exactly one pad space and one
    /// whitespace-free piece — carries a whole-command `lappend`
    /// rewrite fix (the message's own advice made concrete): byte-for-byte
    /// equivalent on a non-empty proper list, and dropping only the
    /// stray leading separator on the first append.  Every other shape
    /// stays message-only: a trailing pad moves the separator to the
    /// other side, several value words or extra padding have no
    /// unambiguous element mapping, a braced value is a deliberate
    /// literal, and a piece with word/list metacharacters would need
    /// requoting.
    pub(in crate::analyser) fn emit_w104_append_list(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        cmd_tok: tcl_lexer::Token,
    ) {
        if cmd_name != "append" || args.len() < 2 || arg_tokens.len() < 2 {
            return;
        }
        for (i, text) in args.iter().enumerate().skip(1) {
            if text.starts_with(' ') || text.ends_with(' ') {
                let tok = arg_tokens.get(i).unwrap_or(&arg_tokens[0]);
                let fixes = self
                    .w104_lappend_fix(args, arg_tokens, arg_expand, cmd_tok)
                    .into_iter()
                    .collect();
                self.result.diagnostics.push(
                    crate::analyser::types::Diagnostic::new(
                        DiagCode::W104,
                        tok.span,
                        "append with space-separated values looks like list \
                              construction. Use [lappend] instead to safely handle values \
                              containing spaces, braces, or backslashes."
                            .to_string(),
                        super::types::Severity::Hint,
                    )
                    .with_fixes(fixes),
                );
                return;
            }
        }
    }

    /// The `lappend` rewrite for W104's mechanical shape (see
    /// [`Self::emit_w104_append_list`]): exactly `append VAR "<sp>PIECE"`
    /// with no `{*}` expansion, a single pad space, and both the
    /// variable word and the piece safe to re-paste as bare words.
    /// The fix spans the whole command — `lappend VAR PIECE`, built
    /// from the raw source slices so the user's spelling is kept.
    fn w104_lappend_fix(
        &self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        cmd_tok: tcl_lexer::Token,
    ) -> Option<super::types::CodeFix> {
        if args.len() != 2 || arg_tokens.len() != 2 || arg_expand.iter().any(|&e| e) {
            return None;
        }
        // Leading-space shape only: `append var "x "` appends the
        // separator *after* the piece, which `lappend` cannot reproduce.
        if !args[1].starts_with(' ') || args[1].ends_with(' ') {
            return None;
        }
        // The value word must be a double-quoted word in the source; a
        // single-fragment quoted token's span end sits on its closing
        // quote (the inner-end convention), so widen over it.
        let vstart = arg_tokens[1].span.start() as usize;
        let mut vend = arg_tokens[1].span.end() as usize;
        if self.source.as_bytes().get(vend) == Some(&b'"') {
            vend += 1;
        }
        let raw = self.source.get(vstart..vend)?;
        let inner = raw.strip_prefix('"')?.strip_suffix('"')?;
        // Exactly one pad space, then one bare-word-safe piece.
        let piece = inner.strip_prefix(' ')?;
        if !is_safe_bare_word(piece) {
            return None;
        }
        let name_tok = arg_tokens[0];
        let name_raw = self
            .source
            .get(name_tok.span.start() as usize..name_tok.span.end() as usize)?;
        if !is_safe_bare_word(name_raw) {
            return None;
        }
        let span = tcl_lexer::Span::new(
            cmd_tok.span.start(),
            u32::try_from(vend).unwrap_or(arg_tokens[1].span.end()),
        );
        Some(super::types::CodeFix {
            span,
            new_text: format!("lappend {name_raw} {piece}"),
            description: "Rewrite with `lappend`".to_string(),
            // W104: `append x " a"` and `lappend x a` agree only when `x`
            // already holds a well-formed list — which depends on its run-time
            // value, not on anything visible here.
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
    }

    /// W106: an unbraced `switch` body undergoes an extra round
    /// of substitution (especially dangerous under `-regexp`).  Handles
    /// the single trailing-body form and the alternating pattern/body
    /// form; skips braced bodies and the `-` fall-through.  ERROR when a
    /// substitution is present or `-regexp` is set, else WARNING.
    pub(in crate::analyser) fn emit_w106_unbraced_switch_body(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((case, invocation)) = registry.case_invocation(
            cmd_name,
            &refs,
            Some(self.analysis_context().context().authoring_query()),
        ) else {
            return;
        };
        if !case.warn_unbraced_bodies {
            return;
        }

        // Single trailing arg: the braced-list form (W105 / bracing
        // handles a braced block); only flag an *unbraced* single block.
        if let Some(i) = invocation.clause_list_index {
            if let Some(tok) = arg_tokens.get(i)
                && !is_braced_word(tok)
            {
                let dangerous = has_substitution(&args[i], tok);
                self.push_w106(
                    tok.span,
                    dangerous,
                    invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp,
                    true,
                );
            }
            return;
        }

        // Alternating pattern/body pairs.
        let Some(i) = invocation.inline_clause_start else {
            return;
        };
        let Some(clauses) = case.inline_clauses(&refs, i) else {
            return;
        };
        for clause in clauses {
            let Some(body_idx) = clause.body_index else {
                continue;
            };
            if let (Some(tok), Some(text)) = (arg_tokens.get(body_idx), args.get(body_idx))
                && !is_braced_word(tok)
                && text != "-"
            {
                let has_regexp = invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp
                    || clause.mode == tcl_registry::spec::CaseMatchMode::Regexp;
                let dangerous = has_substitution(text, tok) || has_regexp;
                self.push_w106(tok.span, dangerous, has_regexp, false);
            }
        }
    }

    /// Push one W106 diagnostic with the message variant selected by
    /// `has_regexp` / substitution danger.
    fn push_w106(
        &mut self,
        span: tcl_lexer::Span,
        dangerous: bool,
        has_regexp: bool,
        single_block: bool,
    ) {
        let message = if has_regexp {
            "switch -regexp body is not braced \u{2014} patterns and actions undergo extra \
             substitution, risking code injection. Use braces: { \u{2026} }"
                .to_string()
        } else if dangerous && single_block {
            "switch body is not braced \u{2014} contains substitutions that risk code \
             injection. Use braces: switch \u{2026} { pattern { body } \u{2026} }"
                .to_string()
        } else if single_block {
            "switch body is not braced \u{2014} Use braces: switch \u{2026} { pattern { body } \
             \u{2026} }"
                .to_string()
        } else if dangerous {
            "switch body is not braced and contains substitutions \u{2014} risk of code \
             injection. Use braces: { \u{2026} }"
                .to_string()
        } else {
            "switch body should be braced to prevent accidental substitution. Use braces: \
             { \u{2026} }"
                .to_string()
        };
        let severity = if dangerous {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W106,
                span,
                message,
                severity,
            ));
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
    /// hand-maintained list — the two previous lists (`name_arg_indices` and
    /// `w216_varname_word_indices`) had drifted, each missing what the other
    /// had. The one structural exception is `upvar`: its frame-linking
    /// semantics are modelled by the registry's repeated layout, and only its
    /// *local*-name slots are name positions — a computed `$remote` in an
    /// other-var slot is a legitimate indirect link.
    pub(in crate::analyser) fn variable_name_positions(
        &self,
        cmd: &str,
        args: &[String],
    ) -> Vec<usize> {
        use tcl_registry::arg_role::ArgRole::{VarRead, VarWrite};
        let Some(registry) = self.registry.as_deref() else {
            return Vec::new();
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        if registry.frame_effect(cmd).is_some_and(|effect| {
            effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
        }) {
            return registry.arg_indices_for_role(cmd, &arg_strs, VarWrite);
        }
        let mut idx = registry.arg_indices_for_role(cmd, &arg_strs, VarWrite);
        idx.extend(registry.arg_indices_for_role(cmd, &arg_strs, VarRead));
        idx.sort_unstable();
        idx.dedup();
        idx
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
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        scope_path: &[usize],
    ) {
        // Lazily materialised: most calls trip no W212, so the scope's
        // variable set is only collected on the first finding.
        let mut scope_vars: Option<Vec<String>> = None;
        for idx in self.variable_name_positions(cmd_name, args) {
            let (Some(tok), Some(text)) = (arg_tokens.get(idx), args.get(idx)) else {
                continue;
            };
            if tok.kind != tcl_lexer::TokenType::Var {
                continue;
            }
            // `set ${token}(status) …` in a variable-name position is the
            // braced indirect-array-element idiom (`token` holds the array
            // name), not a `set $token` dynamic-name foot-gun.  Both the
            // W212 `did you mean token(status)` and the W216
            // `did you mean $token(status)` suggestions are wrong there, so
            // neither fires.  This is the `is_braced_indirect_array_ref`
            // carve-out.
            if tcl_syntax::naming::is_braced_indirect_array_ref(text) {
                continue;
            }
            let bare = text
                .trim_start_matches('$')
                .trim_start_matches('{')
                .trim_end_matches('}');
            // Name the subcommand too for a subcommand-dispatched command
            // (`info exists`, `dict with`, `array set`), so the message reads
            // "'dict with' expects…". Registry-driven — no command name here.
            let display_cmd = self
                .registry
                .as_ref()
                .filter(|_| idx >= 1)
                .and_then(|reg| reg.get(cmd_name))
                .filter(|spec| !spec.subcommands.is_empty())
                .and_then(|spec| {
                    args.first()
                        .filter(|sub| spec.resolve_subcommand(sub).is_some())
                })
                .map_or_else(|| cmd_name.to_string(), |sub| format!("{cmd_name} {sub}"));
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
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W212,
                    tok.span,
                    format!(
                        "'{display_cmd}' expects a variable name, got substitution (${bare}). \
                     Did you mean '{suggestion}'?"
                    ),
                    super::types::Severity::Warning,
                ));
        }
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
        let cmd_name = cmd.texts[0].as_str();
        let args = &cmd.texts[1..];
        let mut varname_word_starts: FxHashSet<u32> = FxHashSet::default();
        for ai in self.variable_name_positions(cmd_name, args) {
            let wi = ai + 1;
            if let Some(tok) = cmd.argv.get(wi) {
                varname_word_starts.insert(tok.span.start());
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
            if text.contains('(') && text.ends_with(')') {
                if let Some(paren_idx) = text.find('(') {
                    let name = &text[..paren_idx];
                    let inner = &text[paren_idx + 1..text.len() - 1];
                    if !name.is_empty() && index_has_substitution(inner) {
                        let corrected = build_w216_replacement(name, inner);
                        // The `}` closing a `${…}` word is the owner
                        // family's call, not `span.end() + 1` — that
                        // overshoots the degenerate `${}` (issue #1423).
                        let span = tcl_lexer::word_span_at(&self.source, t1.span);
                        let message = format!(
                            "`${{{name}({inner})}}` does not substitute `{inner}` \
(the brace form is documented to apply no further substitution to its \
content); use `{corrected}` to access the array element with index substitution"
                        );
                        self.result.diagnostics.push(
                            crate::analyser::types::Diagnostic::new(
                                DiagCode::W216,
                                span,
                                message,
                                Severity::Warning,
                            )
                            .with_fixes(vec![super::types::CodeFix {
                                span,
                                new_text: corrected.clone(),
                                description: format!("Replace with `{corrected}`"),
                                // W216: a `did you mean` reading of an ambiguous reference.
                                safety: crate::irules_checks::FixSafety::RequiresReview,
                            }]),
                        );
                    }
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
            let corrected = build_w216_replacement(&text, inner);
            let span = tcl_lexer::Span::new(
                t1.span.start(),
                u32::try_from(paren_end + 1).unwrap_or(t1.span.end()),
            );
            let message = format!(
                "`${{{text}}}({inner})` is parsed as scalar `${{{text}}}` followed by \
literal text `({inner})`; did you mean `{corrected}` for array element access?"
            );
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W216,
                    span,
                    message,
                    Severity::Warning,
                )
                .with_fixes(vec![super::types::CodeFix {
                    span,
                    new_text: corrected.clone(),
                    description: format!("Replace with `{corrected}`"),
                    // W216: as above.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }]),
            );
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
    /// Fires when at
    /// least one operand of a `==` / `!=` comparison is a string
    /// literal (`ExprString`, e.g. `"foo"`, `"1"`, `"true"`);
    /// comparisons between variables (`$x == $y`) are left alone.
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
        let parsed = crate::parse_expr(trimmed, Some(self.dialect()));
        // ``ExprNode::Raw`` means the expression was unparseable.
        if matches!(parsed, ExprNode::Raw { .. }) {
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
        // Only offer the regex-based code fix when every ``==``/
        // ``!=`` in the expression has a string-literal operand;
        // otherwise the blanket rewrite would incorrectly change
        // non-string comparisons too.
        let total = count_eq_ne_ops(&parsed);
        let mut fixes = Vec::new();
        if matched_ops.len() >= total {
            let rewritten = rewrite_string_compare_ops(expr_text);
            if rewritten != expr_text {
                fixes.push(super::types::CodeFix {
                    span: diag_span,
                    new_text: rewritten,
                    description: format!("Use '{replacement}' for string comparison"),
                    // W110: `eq` compares as strings where `==` coerces numerically —
                    // `"1" == "01"` is true, `"1" eq "01"` is false. Removing the
                    // coercion is the fix, and it changes results in exactly the cases
                    // the diagnostic is about.
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                });
            }
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

/// Render the safe replacement for a W216 array-element reference.  Bare
/// `$name(idx)` is the only `$`-form that substitutes `$` inside the index,
/// so prefer it when `name` allows it; otherwise `[set "name(idx)"]` (the
/// command parser substitutes `$`-vars in `set`'s argument).
fn build_w216_replacement(name: &str, inner: &str) -> String {
    if tcl_syntax::naming::is_bare_var_name(name) {
        format!("${name}({inner})")
    } else {
        format!("[set \"{name}({inner})\"]")
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

/// Integer format specifiers for `binary format` / `binary scan` that
/// accept the Tcl 8.5+ `u` / `s` modifier.
const BINARY_INT_SPECIFIERS: &[u8] = b"csSiInTwWmrR";

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
pub(super) fn is_safe_literal_expr(text: &str, dialect: &str) -> bool {
    use tcl_lexer::ExprTokenType as T;
    if is_safe_literal(text) {
        return true;
    }
    if text.contains('$') || text.contains('[') {
        return false;
    }
    let tokens = tcl_lexer::tokenise_expr(text, Some(dialect));
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
    // flagged (issue #726).
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
/// Derived from `tcl_syntax::expr::operators` (issue #983's unification):
/// every `BinOp` whose mathop shape is a string-only `BoolChain`/`NeBinary`,
/// or `Membership` — rather than a hand-typed 4-entry list that used to
/// miss `lt`/`le`/`gt`/`ge` entirely, a real gap since those four share
/// exactly the same numeric-normalisation risk `eq`/`ne` already guarded
/// against (`{[expr {$x}] lt "007"}` could have its verdict silently
/// flipped by W114's unwrap fix the same way `==`'s can).
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

/// Walk `node` and collect every `==`/`!=` operator whose at least
/// one operand is a string literal ([`ExprNode::String`]), paired with
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
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
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
            if matches!(op, BinOp::Eq | BinOp::Ne)
                && (matches!(**left, ExprNode::String { .. })
                    || matches!(**right, ExprNode::String { .. }))
            {
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
    // Entry point: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`node_extent_at`]).
    node_extent_at(node, 0)
}

fn node_extent_at(node: &ExprNode, depth: u32) -> Option<(u32, u32)> {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
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
        ExprNode::Raw { .. } => None,
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

/// Count the total number of `==`/`!=` operators in the expression
/// tree.
fn count_eq_ne_ops(node: &ExprNode) -> usize {
    // Entry point: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`count_eq_ne_ops_at`]).
    count_eq_ne_ops_at(node, 0)
}

fn count_eq_ne_ops_at(node: &ExprNode, depth: u32) -> usize {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop counting (return 0 for the
    // deeper sub-tree) — a conservative under-count only reachable past 256
    // levels of expression nesting; never a crash.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return 0;
    }
    match node {
        ExprNode::Binary { op, left, right } => {
            let mut n = count_eq_ne_ops_at(left, depth + 1) + count_eq_ne_ops_at(right, depth + 1);
            if matches!(op, BinOp::Eq | BinOp::Ne) {
                n += 1;
            }
            n
        }
        ExprNode::Unary { operand, .. } => count_eq_ne_ops_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            count_eq_ne_ops_at(condition, depth + 1)
                + count_eq_ne_ops_at(true_branch, depth + 1)
                + count_eq_ne_ops_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().map(|a| count_eq_ne_ops_at(a, depth + 1)).sum(),
        _ => 0,
    }
}

/// Rewrite `==`/`!=` operators to ` eq `/` ne ` for use in a code
/// fix's replacement text.
///
/// The rewrite rules are:
/// * `(?<![=!])==(?!=)`  → ` eq `
/// * `!=`                → ` ne `
/// * `[ \t]{2,}`         → ` `  (collapse runs of 2+ ws)
fn rewrite_string_compare_ops(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut step1 = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // !=  →  " ne "
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '=' {
            step1.push_str(" ne ");
            i += 2;
            continue;
        }
        // ==  →  " eq "  (with negative look-around)
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            let prev_ok = i == 0 || (chars[i - 1] != '=' && chars[i - 1] != '!');
            let next_ok = i + 2 >= chars.len() || chars[i + 2] != '=';
            if prev_ok && next_ok {
                step1.push_str(" eq ");
                i += 2;
                continue;
            }
        }
        step1.push(c);
        i += 1;
    }
    // Collapse runs of 2+ space/tab into a single space.  Single
    // whitespace characters are preserved.
    let chars: Vec<char> = step1.chars().collect();
    let mut out = String::with_capacity(step1.len());
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == ' ' || chars[i] == '\t')
            && i + 1 < chars.len()
            && (chars[i + 1] == ' ' || chars[i + 1] == '\t')
        {
            out.push(' ');
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod issue996_tests {
    use super::*;
    use crate::expr_ast::UnaryOp;

    /// Regression coverage for issue #996: `walk_string_eq_ne`, `node_extent`
    /// and `count_eq_ne_ops` each recurse once per `ExprNode` level with no
    /// depth cap before this fix (`walk_string_eq_ne` also drives
    /// `node_extent` via `op_offset_between`). A tree built directly is
    /// unbounded (the Pratt parser caps its own output at 256) and
    /// empirically overflowed the native stack (SIGABRT) in the low thousands
    /// of levels on a 2 MiB thread. 3000 is past that crash range and past
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
        let _ = count_eq_ne_ops(&node);
        let _ = node_extent(&node);
    }
}
