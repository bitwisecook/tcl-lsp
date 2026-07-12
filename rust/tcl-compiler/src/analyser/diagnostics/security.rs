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

//! Injection and unsafe-evaluation security checks emitted during the
//! command walk.
//!
//! These diagnostics flag constructs that splice attacker-influenced data
//! into a command, an `eval`/`uplevel`/`interp eval` target, a `subst`
//! template, or a regular expression: string-concatenated `eval` (W101),
//! `subst` injection (W102), an open `exec`/`open` pipeline (W103), a
//! `catch` with no result variable that hides a failure (W302), a regex
//! prone to catastrophic backtracking (W303), a non-literal command word
//! where a literal is expected (W306), a `source` of a variable path
//! (W300), `uplevel` / `interp eval` injection (W301, W312),
//! double-decoded `eval [subst …]` (W309), closed value arguments left
//! open (W127), and a hardcoded credential literal (W310).

use super::helpers::{has_substitution, is_braced_word};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::regex_source::regexp_pattern_index;
use tcl_core_types::DiagCode;

impl Analyser {
    /// **W302.** Emit "catch without result variable" hint when a
    /// `catch BODY` invocation omits the optional `RESULTVAR`
    /// argument, silently swallowing any error the body raises.
    ///
    /// W302 is emitted for `IRCatch` (not `IRBarrier`) — the lowerer
    /// falls back to `IRBarrier` when the body argument is multi-token
    /// (e.g. ``catch $body``), so this emit gates on ``arg_single[0]``
    /// to suppress that case.  The diagnostic anchors at just the
    /// ``catch`` command token — the narrowest span that identifies
    /// the issue.
    pub(in crate::analyser) fn emit_w302_catch_no_result_var(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        // Only fires when a result variable is absent.  Empty args
        // is a malformed catch (IRBarrier path,
        // no W302).  ≥2 args means a result variable is present.
        if args.len() != 1 {
            return;
        }
        // Suppress on a catch with a dynamic body: a multi-token body
        // word can't be statically resolved to a script, so the lowerer
        // drops it before the statement check ever sees it.
        if arg_single.first().copied() != Some(true) {
            return;
        }
        if arg_tokens.is_empty() {
            return;
        }
        // Suppress the hint on the documented "fire-and-forget" idiom:
        // ``catch {close $h}`` / ``catch {after cancel $h}`` etc.  These
        // commands error when the target is already gone, and a bare
        // ``catch {<cmd>}`` is the canonical Tcl idiom for "do this if
        // possible, ignore if not".
        if let Some(body) = args.first()
            && catch_body_is_fire_and_forget(body)
        {
            return;
        }
        let span = cmd_tok.span;
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W302,
            span,
            message: "catch without a result variable silently swallows errors. \
Consider capturing the result: catch {\u{2026}} result"
                .to_string(),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **W101.** Emit "eval with string concatenation" warning
    /// when an `eval` invocation's argument list could be a
    /// substitution-driven injection vector.
    ///
    /// Suppressed when:
    ///
    /// - every argument's representative token is `Str` (braced,
    ///   `eval {script}` / `eval {a} {b}` — the safe form), or
    /// - the single argument is a `Cmd` substitution whose inner
    ///   command head produces a canonical list (per
    ///   [`tcl_registry::CommandRegistry::is_canonical_list_command`]
    ///   — `eval [list ...]`, `eval [linsert ...]`, etc.).
    ///
    /// Otherwise fires `Severity::Warning` when any argument's
    /// representative token is `Var` / `Cmd` (substitution at the
    /// word level), or any argument is a multi-token word
    /// (substitution within the word — the single-token-word flag
    /// is `false`).  This is a sound approximation of the
    /// `all_tokens[1:]`-walk: `process_command` doesn't currently
    /// thread the full token stream, but multi-token-word implies
    /// inner substitution and the per-arg representative kind
    /// covers the single-token VAR / CMD cases.
    ///
    /// Diagnostic anchors at the first argument's range.
    pub(in crate::analyser) fn emit_w101_eval_string_concat(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "eval" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // ``eval {script}`` / ``eval {a} {b}`` — every word is a
        // braced literal, no substitution risk.
        if arg_tokens
            .iter()
            .all(|tok| matches!(tok.kind, tcl_lexer::TokenType::Str))
        {
            return;
        }
        // ``eval [list ...]`` and similar canonical-list idioms —
        // single-arg ``Cmd`` whose inner head produces a canonical
        // list.
        if arg_tokens.len() == 1 && self.is_canonical_list_substitution(arg_tokens[0]) {
            return;
        }
        // Substitution detection.  An argument carries substitution
        // when:
        //
        // - the representative token kind is ``Var`` / ``Cmd``
        //   (single-token substitution at the word level), or
        // - the word is multi-token AND its source range contains
        //   an unescaped ``$`` / ``[`` outside any ``{...}`` block.
        //
        // The multi-token-word flag alone is **not** equivalent to
        // substitution: the segmenter sets ``single_token_word=false``
        // for any adjacent-token concatenation, including pure-
        // literal shapes like ``eval foo{bar}`` (Esc+Str joined,
        // no inner Var/Cmd).  An
        // ``all_tokens[1:]`` walk would require threading the full
        // token stream through ``process_command``; instead we do a
        // brace/backslash-aware source-byte scan over the word's
        // span, which is sound for the common cases.  Known
        // approximation gap: ``"foo{$x}bar"`` (substitution inside
        // a brace pair within a quoted string — Tcl treats braces
        // as literal inside ``"…"``) is not detected.  Real W101
        // shapes don't hit that pattern; documented for posterity.
        // Anchor at the *first argument that actually carries the
        // substitution*, not `arg_tokens[0]`: `eval "safeprefix" $x` puts the
        // hazard in `$x`, so highlighting the safe literal prefix would point
        // the developer at the wrong word.
        let Some(sub_idx) = arg_tokens.iter().enumerate().position(|(i, tok)| {
            if matches!(
                tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            ) {
                return true;
            }
            if arg_single.get(i).copied() == Some(true) {
                return false;
            }
            self.word_span_contains_substitution(tok.span)
        }) else {
            return;
        };
        let anchor = arg_tokens[sub_idx];
        // Quick-fix the common single-line `eval "cmd $a …"` shape: rewrite
        // the quoted string to `eval [list cmd $a …]`.  `[list]` builds a
        // properly-quoted list so each substituted word is passed as exactly
        // one argument and never re-parsed.  Skip when the string spans
        // lines or carries backslash escapes (list re-quoting could differ).
        // Only offered when the substituted word is itself the quoted string
        // (`eval_list_fix` returns empty otherwise).
        let fixes = self.eval_list_fix(anchor);
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W101,
            span: anchor.span,
            message: "eval with substituted arguments risks code injection. \
Prefer direct invocation or {*}$cmdList to preserve argument boundaries."
                .to_string(),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Build the `eval [list …]` rewrite fix for a W101 diagnostic whose
    /// `eval` argument is a single-line double-quoted string
    /// (`eval "cmd $a"` → `eval [list cmd $a]`).  Returns an empty vec for
    /// any other shape (braced, multi-line, backslash-escaped).
    fn eval_list_fix(&self, first: tcl_lexer::Token) -> Vec<super::types::CodeFix> {
        let bytes = self.source.as_bytes();
        let open = first.span.start() as usize;
        if bytes.get(open) != Some(&b'"') {
            return Vec::new();
        }
        let Some(rel_close) = self.source[open + 1..].find('"') else {
            return Vec::new();
        };
        let close = open + 1 + rel_close;
        let inner = &self.source[open + 1..close];
        if inner.is_empty() || inner.contains('\n') || inner.contains('\\') {
            return Vec::new();
        }
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(first.span.start(), u32::try_from(close + 1).unwrap_or(0)),
            new_text: format!("[list {inner}]"),
            description: "Rewrite to `eval [list …]` (passes each substituted \
word as one argument; no re-parsing)"
                .to_string(),
        }]
    }

    /// Scan the source bytes covered by `span` for an unescaped
    /// ``$`` or ``[`` outside any ``{...}`` brace block.  Used by
    /// [`Self::emit_w101_eval_string_concat`] to detect inner
    /// substitution within a multi-token word without requiring
    /// the full token stream to be threaded through
    /// ``process_command``.
    ///
    /// Brace tracking: ``{`` increments depth, ``}`` decrements;
    /// ``$`` / ``[`` only count as substitution when depth is
    /// zero.  Backslash escapes consume the next byte (so ``\$``
    /// is skipped).  Out-of-bounds spans return false rather than
    /// panicking.
    fn word_span_contains_substitution(&self, span: tcl_lexer::Span) -> bool {
        let start = span.start() as usize;
        let end = span.end() as usize;
        if end > self.source.len() || start >= end {
            return false;
        }
        let bytes = self.source.as_bytes();
        let mut i = start;
        let mut brace_depth: i32 = 0;
        while i < end {
            match bytes[i] {
                b'\\' if i + 1 < end => {
                    i += 2;
                    continue;
                }
                b'{' => brace_depth += 1,
                b'}' if brace_depth > 0 => brace_depth -= 1,
                b'$' | b'[' if brace_depth == 0 => return true,
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Helper for [`Self::emit_w101_eval_string_concat`].  Returns
    /// true when `tok` is a `Cmd` token whose inner script's
    /// command head (or `cmd subcmd` pair) produces a canonical
    /// list per the registry — the W101 safe-idiom suppression.
    ///
    /// Conservative: rejects multi-command scripts (containing `;`
    /// or newline) because `[list a b; set x $user]` returns the
    /// last command's result, which isn't necessarily a safe list.
    fn is_canonical_list_substitution(&self, tok: tcl_lexer::Token) -> bool {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return false;
        }
        let Some(registry) = self.registry.as_ref() else {
            return false;
        };
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return false;
        }
        let script = self.source[start..end].trim();
        if script.is_empty() || script.contains(';') || script.contains('\n') {
            return false;
        }
        // ``parts[0]`` = command head; check both bare form and
        // ``"head sub"`` compound form.
        let mut iter = script.splitn(2, char::is_whitespace);
        let Some(head) = iter.next() else {
            return false;
        };
        if registry.is_canonical_list_command(head) {
            return true;
        }
        if let Some(rest) = iter.next() {
            let mut sub_iter = rest.trim_start().splitn(2, char::is_whitespace);
            if let Some(sub) = sub_iter.next() {
                let compound = format!("{head} {sub}");
                if registry.is_canonical_list_command(&compound) {
                    return true;
                }
            }
        }
        false
    }

    /// Shared substitution probe for the W3xx injection checks:
    /// returns `true` when any argument word carries a `$var` / `[cmd]`
    /// substitution.  A word counts as substituted when its
    /// representative token is `Var` / `Cmd` (single-token substitution
    /// at the word level) or — for a multi-token word — its source span
    /// contains an unescaped `$` / `[` outside any `{...}` block.  This
    /// is the same approximation of the `all_tokens[1:]` walk that
    /// [`Self::emit_w101_eval_string_concat`] uses (the analyser doesn't
    /// thread the full token stream through `process_command`); the
    /// representative-kind + brace-aware span scan covers every shape in
    /// the security fixtures.
    fn args_have_substitution(&self, arg_tokens: &[tcl_lexer::Token], arg_single: &[bool]) -> bool {
        arg_tokens.iter().enumerate().any(|(i, tok)| {
            if matches!(
                tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            ) {
                return true;
            }
            if arg_single.get(i).copied() == Some(true) {
                return false;
            }
            self.word_span_contains_substitution(tok.span)
        })
    }

    /// Return the trimmed inner script of a `Cmd` substitution token
    /// (`[ … ]` with the brackets stripped), or `None` for a non-`Cmd`
    /// token or an out-of-bounds span.
    fn cmd_token_inner(&self, tok: tcl_lexer::Token) -> Option<&str> {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return None;
        }
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return None;
        }
        Some(self.source[start..end].trim())
    }

    /// **W300.** Warn when `source`'s file argument is a `$var` or
    /// `[cmd]` substitution — the path (and therefore the code executed)
    /// is dynamic.  Skips a leading `-encoding ENC` option pair.
    /// When `tok` is a `$var` reference whose value provably resolves to a
    /// compile-time *literal* — a local `set var LITERAL` reaching this use,
    /// per [`super::validity::last_literal_set_value_for_var`] — return that
    /// literal. A parameter, a dynamic value, a `[cmd]` substitution, or a
    /// cross-scope name yields `None`.
    ///
    /// The resolver only accepts a genuine literal value token (`Esc` / `Str`,
    /// never `$x` / `[cmd]`), so the returned string is provably not
    /// attacker-influenced. That is exactly what the `source` (W300) and `open`
    /// (W103) sink checks need to drop their false positives: a value that is a
    /// known constant is no more dangerous than writing that constant inline.
    fn resolve_var_token_literal(&self, tok: tcl_lexer::Token) -> Option<String> {
        if tok.kind != tcl_lexer::TokenType::Var {
            return None;
        }
        let name = self.var_name_from_token(tok)?;
        super::validity::last_literal_set_value_for_var(
            &self.source,
            &name,
            tok.span.start(),
            self.lexer_config(),
        )
        .map(|(literal, _, _)| literal)
    }

    pub(in crate::analyser) fn emit_w300_source_variable(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "source" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let mut file_idx = 0;
        if args[0] == "-encoding" && args.len() >= 3 {
            file_idx = 2;
        }
        let Some(&tok) = arg_tokens.get(file_idx) else {
            return;
        };
        // A `$var` path or a `[cmd]`-computed path is equally dynamic — both
        // execute whatever file the value resolves to.
        if matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) {
            // …unless the `$var` provably holds a compile-time literal path, in
            // which case it is a known file — the same as `source ./lib.tcl`,
            // which is silent — so flagging it is a false positive.
            if self.resolve_var_token_literal(tok).is_some() {
                return;
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W300,
                span: tok.span,
                message: "source with a dynamic path (variable or command substitution) \
executes arbitrary Tcl code. Ensure the path is not influenced by untrusted input."
                    .to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W309.** Emit "eval/uplevel with `[subst]` — double
    /// substitution" when an `eval` / `uplevel` argument is a `[subst …]`
    /// command substitution: `subst` expands `$var` / `[cmd]` once, then
    /// the outer command re-parses the result as Tcl — a classic
    /// double-decode injection.  One diagnostic is emitted per command.
    /// Approximation: only the per-word
    /// representative `Cmd` tokens are scanned, so a `[subst …]` buried
    /// inside a larger quoted word isn't detected (same limitation as
    /// W101 / W309 in the absence of the full token stream).
    pub(in crate::analyser) fn emit_w309_eval_subst_double_decode(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "eval" | "uplevel") || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        for tok in arg_tokens {
            let Some(inner) = self.cmd_token_inner(*tok) else {
                continue;
            };
            if inner == "subst" || inner.starts_with("subst ") || inner.starts_with("subst\t") {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W309,
                    span: tok.span,
                    message: format!(
                        "{cmd_name} with [subst] creates double substitution: \
subst expands $var and [cmd], then {cmd_name} re-parses the result as Tcl. \
This is a code-injection risk. Use [format] or [string map] for safe templating."
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }

    /// **W301.** Emit "uplevel with string-built script" when an
    /// `uplevel` script argument risks injection: either multiple script
    /// arguments (concatenated like `eval`) or a single unbraced script
    /// word carrying substitution.  Skips a leading `?level?` argument;
    /// the `[list …]` idiom is the recognised safe form.
    pub(in crate::analyser) fn emit_w301_uplevel_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "uplevel" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let script_idx = usize::from(uplevel_has_level(&args[0]));
        if script_idx >= args.len() || script_idx >= arg_tokens.len() {
            return;
        }
        let remaining = &args[script_idx..];
        let remaining_toks = &arg_tokens[script_idx..];
        if remaining.len() > 1 {
            // Multiple args = concat behaviour = danger.
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W301,
                    span: remaining_toks[0].span,
                    message: "uplevel with multiple arguments concatenates them into \
a script (like eval). Use a single braced body or {*}$cmdList to avoid injection."
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        } else if let Some(tok) = remaining_toks.first() {
            // Single arg — unbraced + substituted (and not [list …]).
            if matches!(tok.kind, tcl_lexer::TokenType::Str)
                || self.is_canonical_list_substitution(*tok)
            {
                return;
            }
            // A single *pure* variable substitution (`uplevel 1 $body`) is the
            // safe single-substitution idiom: tclsh evaluates `$body` once in
            // the target frame, no concatenation / second substitution.  The
            // script word must be exactly one `Var` token — a concatenation
            // (`$a$b`, `pre$x`) is not a single token and stays flagged.
            if arg_single.get(script_idx).copied() == Some(true)
                && matches!(tok.kind, tcl_lexer::TokenType::Var)
            {
                return;
            }
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W301,
                    span: tok.span,
                    message: "uplevel with an unbraced script argument may cause \
double substitution. Use braces: uplevel 1 {...}"
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W312.** Emit "interp eval / invokehidden injection" when an
    /// `interp eval` / `interp invokehidden` script argument risks
    /// injection — the same shape as W301 but for the child-interpreter
    /// dispatch.
    pub(in crate::analyser) fn emit_w312_interp_eval_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "interp" || args.is_empty() {
            return;
        }
        let sub = args[0].as_str();
        if !matches!(sub, "eval" | "invokehidden") || args.len() < 3 || arg_tokens.len() < 3 {
            return;
        }
        // Locate the first script word: `interp eval PATH script …` →
        // index 2; `interp invokehidden PATH ?-opt…? hiddenCmd …` → the
        // first non-option word from index 2.
        let script_start = if sub == "eval" {
            2
        } else {
            let mut i = 2;
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            if i >= args.len() {
                return;
            }
            i
        };
        if script_start >= args.len() || script_start >= arg_tokens.len() {
            return;
        }
        let script_args = &args[script_start..];
        let script_toks = &arg_tokens[script_start..];
        if script_args.is_empty() || script_toks.is_empty() {
            return;
        }
        // `interp eval` with multiple script words concatenates them.
        if sub == "eval" && script_args.len() > 1 {
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W312,
                    span: script_toks[0].span,
                    message: format!(
                        "interp {sub} with multiple arguments concatenates \
them into a script (like eval). Use a single braced body to avoid injection."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
            return;
        }
        let tok = script_toks[0];
        if matches!(tok.kind, tcl_lexer::TokenType::Str) || self.is_canonical_list_substitution(tok)
        {
            return;
        }
        if self.args_have_substitution(arg_tokens, arg_single) {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W312,
                span: tok.span,
                message: format!(
                    "interp {sub} with an unbraced script argument may \
cause code injection. Use braces: interp {sub} $child {{...}}"
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W102.** Emit "subst on variable input" when `subst`'s template
    /// argument is a bare `$var` substitution — `subst` performs `$` /
    /// `[]` substitution on its argument, so a variable template enables
    /// code injection.  The message lists
    /// exactly the substitution kinds still active (`-nocommands` /
    /// `-novariables` narrow it) and is suppressed entirely when both
    /// flags are present (only backslash substitution remains).
    pub(in crate::analyser) fn emit_w102_subst_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "subst" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let (template_idx, nocommands, novariables) = parse_subst_flags(args);
        let Some(idx) = template_idx else {
            return;
        };
        let Some(tok) = arg_tokens.get(idx) else {
            return;
        };
        if !matches!(tok.kind, tcl_lexer::TokenType::Var) {
            return;
        }
        if nocommands && novariables {
            // Only backslash substitution remains — low risk.
            return;
        }
        let mut active = String::new();
        if !nocommands {
            active.push_str("[cmd]");
        }
        if !nocommands && !novariables {
            active.push_str(" and ");
        }
        if !novariables {
            active.push_str("$var");
        }
        let mut mitigations: Vec<&str> = Vec::new();
        if !nocommands {
            mitigations.push("-nocommands");
        }
        if !novariables {
            mitigations.push("-novariables");
        }
        let message = format!(
            "subst with a variable argument enables code injection: any \
{active} in the string will be evaluated. Add {} to limit substitution \
scope, or use [format] / [string map] for safe templating.",
            mitigations.join(" ")
        );
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W102,
            span: tok.span,
            message,
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W103.** Emit "open with a pipeline" when `open`'s first
    /// argument requests a command pipeline (`open "|cmd"`) or is a
    /// variable that might.  A `|`-prefixed
    /// argument carrying substitution is a WARNING (command injection),
    /// a literal `|`-pipeline is a HINT, and a bare `$var` argument is a
    /// WARNING (it may resolve to a `|`-pipeline).
    pub(in crate::analyser) fn emit_w103_open_pipeline(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "open" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let tok = arg_tokens[0];
        if args[0].starts_with('|') {
            let (severity, message) = if self.args_have_substitution(arg_tokens, arg_single) {
                (
                    Severity::Warning,
                    "open with a pipeline containing variable/command \
substitution risks command injection. Validate and sanitize the command \
before passing to open."
                        .to_string(),
                )
            } else {
                (
                    Severity::Hint,
                    "open with a pipeline (\"|\") executes an external command. \
Ensure the command is not influenced by untrusted input."
                        .to_string(),
                )
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W103,
                span: tok.span,
                message,
                severity,
                fixes: Vec::new(),
            });
        } else if matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) {
            // A `$var` proven to hold a compile-time literal is treated exactly
            // like that literal written inline: a `|`-prefixed literal is the
            // pipeline Hint (a known command, not untrusted), any other literal
            // is a benign filename (silent). Only a genuine literal suppresses
            // the Warning — a parameter, a dynamic value, or a `[cmd]`-computed
            // argument stays a Warning (it may resolve to a `|`-pipeline).
            if let Some(literal) = self.resolve_var_token_literal(tok) {
                if literal.starts_with('|') {
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: DiagCode::W103,
                        span: tok.span,
                        message: "open with a pipeline (\"|\") executes an external command. \
Ensure the command is not influenced by untrusted input."
                            .to_string(),
                        severity: Severity::Hint,
                        fixes: Vec::new(),
                    });
                }
                return;
            }
            // A `$var` or `[cmd]`-computed first argument is equally dynamic —
            // either may resolve to a `|`-prefixed pipeline.
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W103,
                span: tok.span,
                message: "open with a dynamic argument (variable or command substitution): \
if the value starts with \"|\", it will execute a command pipeline. Validate input \
or use explicit I/O commands."
                    .to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W303.** Emit "regexp vulnerable to catastrophic backtracking
    /// (`ReDoS`)" when a *literal* regex pattern in `regexp` / `regsub` /
    /// `switch -regexp` contains a nested quantifier (`(a+)+`) or an
    /// overlapping alternation (`(a|a)+`).  Variable / command-substituted
    /// patterns are left alone (the literal text never matches the
    /// detector).
    pub(in crate::analyser) fn emit_w303_redos(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "regexp" | "regsub" | "switch") {
            return;
        }
        let patterns = find_regex_patterns_in_command(cmd_name, args, arg_tokens);
        if patterns.is_empty() {
            return;
        }
        // Nested quantifier `…+)+` / `…*)*` or overlapping alternation
        // `(a|a)+`.
        for (pattern, tok) in patterns {
            if has_redos_shape(&pattern) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W303,
                    span: tok.span,
                    message: "Regular expression may be vulnerable to catastrophic \
backtracking (ReDoS). Nested quantifiers like (a+)+ can cause exponential \
matching time on crafted input."
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W306.** Warn when a `regexp` / `regsub` *pattern* — a
    /// literal-expected position — contains a *live* substitution Tcl
    /// expands before the regex engine sees it.  A pattern that is exactly
    /// one variable substitution — bare `$var` / `${var}` **or** quoted
    /// `"$var"` (the quotes group nothing, so it is byte-for-byte identical
    /// to the bare form) — is the canonical parameterised-pattern idiom and
    /// is exempt: no literal was "expected" there, and the `{…}` rewrite
    /// would change it to match the literal text `$var`.  A quoted `"[cmd]"`
    /// or an unbraced `[cmd]` computes the pattern dynamically and is the
    /// foot-gun.  `\[` / `\$` in a quoted pattern are literal regex
    /// characters, not substitutions.
    pub(in crate::analyser) fn emit_w306_literal_expected(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "regexp" | "regsub") {
            return;
        }
        let Some(idx) = regexp_pattern_index(args) else {
            return;
        };
        let (Some(&tok), Some(text)) = (arg_tokens.get(idx), args.get(idx)) else {
            return;
        };
        if is_braced_word(&tok) || !has_substitution(text, &tok) {
            return;
        }
        let start = tok.span.start() as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return;
        }
        let is_subst_token = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        // A quoted/literal token (not a single `$var` / `[cmd]` word) only
        // counts when the raw source carries a *live* (unescaped) `[`/`$`.
        if !is_subst_token && !raw_has_live_substitution(&self.source[start..end]) {
            return;
        }
        // Bare `$var` / `${var}` is the canonical idiom — a `Var` word has
        // no surrounding literal text, so it is exactly that form.
        if tok.kind == tcl_lexer::TokenType::Var {
            return;
        }
        // A *quoted* word that is exactly one pure `$var` / `${var}`
        // substitution (`"$pat"`) is byte-for-byte identical at runtime to the
        // bare `$var` exempted above — the quotes group nothing — so it is the
        // same parameterised-pattern idiom, not a foot-gun. `"[cmd]"` (a
        // command substitution) is *not* exempt.
        let inner = text
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(text);
        if crate::value_shapes::is_pure_var_ref(inner) {
            return;
        }
        let is_quoted = self.source.as_bytes().get(start) == Some(&b'"');
        let found = if text.contains('$') { "'$'" } else { "'['" };
        let advice = if is_quoted {
            ". Use braces '{...}' instead of quotes."
        } else {
            ". Use braces '{...}' to prevent substitution."
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W306,
            span: tok.span,
            message: format!(
                "Literal expected in {cmd_name} pattern \u{2014} found {found}{advice}"
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W127.** A literal at a closed-value argument index is not in the
    /// command's allowed set.  Only fires
    /// for indices the spec marks `closed_value_args` (the `arg_values`
    /// are exhaustive, not hints).  Dynamic values (`$var` / `[cmd]`) and
    /// declared option flags are skipped.
    pub(in crate::analyser) fn emit_w127_closed_value_args(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        cmd_tok: tcl_lexer::Token,
    ) {
        let mut hits: Vec<(String, tcl_lexer::Span)> = Vec::new();
        if let Some(registry) = self.registry.as_ref() {
            let Some(spec) = registry.get(cmd_name) else {
                return;
            };
            let span_at = |i: usize| arg_tokens.get(i).map_or(cmd_tok.span, |t| t.span);
            // Top-level closed value args (exact match).
            let opt_names: std::collections::HashSet<&str> =
                spec.options.iter().map(|o| o.name).collect();
            for &idx in spec.closed_value_args {
                let i = idx as usize;
                let Some(value) = args.get(i) else { continue };
                let allowed: Vec<&str> =
                    spec.arg_values_at(idx).iter().map(|av| av.value).collect();
                if let Some(hit) =
                    w127_closed_hit(value, span_at(i), &allowed, false, &opt_names, cmd_name)
                {
                    hits.push(hit);
                }
            }
            // Subcommand-level closed value args: the subcommand word occupies
            // arg 0, so the command-level index is one past the subcommand
            // relative index. `string is <class>` marks its class (`&[0]`),
            // matched by unique prefix (`arg_values_accept_prefix`).
            if let Some(sub) = args.first().and_then(|s| spec.resolve_subcommand(s)) {
                let sub_opt_names: std::collections::HashSet<&str> =
                    sub.options.iter().map(|o| o.name).collect();
                let display = format!("{cmd_name} {}", args[0]);
                for &sub_idx in sub.closed_value_args {
                    let i = sub_idx as usize + 1;
                    let Some(value) = args.get(i) else { continue };
                    let allowed: Vec<&str> = sub
                        .arg_values_at(sub_idx)
                        .iter()
                        .map(|av| av.value)
                        .collect();
                    if let Some(hit) = w127_closed_hit(
                        value,
                        span_at(i),
                        &allowed,
                        sub.arg_values_accept_prefix,
                        &sub_opt_names,
                        &display,
                    ) {
                        hits.push(hit);
                    }
                }
            }
        }
        for (message, span) in hits {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W127,
                span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W127 (option-value sibling).** A literal value word of a value-taking
    /// option whose value set is *closed* (`OptionValue::enumerated(.., true,
    /// ..)`) is not in that set.  Options are matched by name or alias, arity
    /// and the `--` terminator are honoured via `value_indices`, and dynamic
    /// values (`$var` / `[cmd]`) are skipped — mirroring the positional path
    /// without overloading it.
    pub(in crate::analyser) fn emit_w127_closed_option_values(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        cmd_tok: tcl_lexer::Token,
    ) {
        let mut hits: Vec<(String, tcl_lexer::Span)> = Vec::new();
        if let Some(registry) = self.registry.as_ref() {
            let Some(spec) = registry.get(cmd_name) else {
                return;
            };
            let find_opt = |arg: &str| {
                spec.options
                    .iter()
                    .chain(spec.command_forms.iter().flat_map(|f| f.options.iter()))
                    .find(|o| o.matches(arg))
            };
            let mut i = 0usize;
            while i < args.len() {
                let arg = args[i].as_str();
                if arg == "--" {
                    break;
                }
                let Some(opt) = find_opt(arg) else {
                    i += 1;
                    continue;
                };
                let vals = opt.value_indices(args, i);
                let allowed = opt.value_values();
                if opt.value_is_closed() && !allowed.is_empty() {
                    for &vi in &vals {
                        let Some(value) = args.get(vi) else { continue };
                        if value.contains('$')
                            || value.contains('[')
                            || allowed.iter().any(|av| av.value == value.as_str())
                        {
                            continue;
                        }
                        let allowed_list = allowed
                            .iter()
                            .map(|av| av.value)
                            .collect::<Vec<_>>()
                            .join(", ");
                        let span = arg_tokens.get(vi).map_or(cmd_tok.span, |t| t.span);
                        hits.push((
                            format!(
                                "Invalid value '{value}' for option '{arg}' on '{cmd_name}'; expected one of: {allowed_list}"
                            ),
                            span,
                        ));
                    }
                }
                i += 1 + vals.len();
            }
        }
        for (message, span) in hits {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W127,
                span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W310.** Emit "hardcoded credential" for a literal secret value.
    /// Two strategies, one diagnostic per command:
    ///
    /// * **Strategy 1** — a credential-bearing option flag (the defaults
    ///   `-password` / `-pass` / `-secret` / `-token` / `-apikey`,
    ///   case-insensitive, unioned with the command's registry
    ///   `credential_options`, e.g. `http::geturl`'s `-headers`) followed
    ///   by a literal value.
    /// * **Strategy 2** — a subcommand whose registry `credential_arg` /
    ///   `sensitive_headers` mark a literal value at a sensitive header
    ///   (e.g. `HTTP::header insert authorization "Bearer …"`).
    pub(in crate::analyser) fn emit_w310_hardcoded_credentials(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // Registry-augmented credential option flags (all `'static`, so
        // the `self.registry` borrow ends with this binding).
        let extra_opts: &'static [&'static str] = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .map_or(&[], |s| s.credential_options);

        // Strategy 1: a credential option flag with a literal value.
        for (i, text) in args.iter().enumerate() {
            let lower = text.to_ascii_lowercase();
            if !DEFAULT_PASSWORD_OPTIONS.contains(&lower.as_str())
                && !extra_opts.contains(&lower.as_str())
            {
                continue;
            }
            let (Some(value), Some(val_tok)) = (args.get(i + 1), arg_tokens.get(i + 1)) else {
                continue;
            };
            if is_literal_credential_value(value, val_tok) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W310,
                    span: val_tok.span,
                    message: format!(
                        "Hardcoded credential in {text} argument. Store secrets in \
environment variables or a vault, not in source code."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
                return; // one diagnostic per command
            }
        }

        // Strategy 2: a subcommand credential header with a literal value.
        if args.len() >= 3 {
            let sub = args[0].to_ascii_lowercase();
            // `(credential_arg, sensitive_headers)` — both copied out so
            // the registry borrow ends before we mutate `self.result`.
            let cred_info: Option<(usize, &'static [&'static str])> = self
                .registry
                .as_ref()
                .and_then(|r| r.get(cmd_name))
                .and_then(|s| s.resolve_subcommand(&sub))
                .and_then(|sc| {
                    sc.credential_arg
                        .map(|a| (a as usize, sc.sensitive_headers))
                });
            if let Some((cred_arg, sensitive)) = cred_info {
                let header_name = args[1].to_ascii_lowercase();
                if sensitive.contains(&header_name.as_str())
                    && cred_arg < arg_tokens.len()
                    && let (Some(value), Some(val_tok)) =
                        (args.get(cred_arg), arg_tokens.get(cred_arg))
                    && is_literal_credential_value(value, val_tok)
                {
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: DiagCode::W310,
                        span: val_tok.span,
                        message: format!(
                            "Hardcoded credential in {header_name} header value. \
Store secrets in environment variables or a vault, not in source code."
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }
}

/// True when `pattern` contains a catastrophic-backtracking shape: a
/// nested quantifier (`…+)+`, `…*)*`, `…+){`) or an overlapping
/// alternation (`(…|…)` immediately followed by `+` / `*` / `{`).
/// Hand-written replacement for the `_REDOS_PATTERN` regex.
pub(super) fn has_redos_shape(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let quant = |b: Option<&u8>| matches!(b, Some(b'+' | b'*' | b'{'));
    // Nested quantifier: `<+|*> ) <+|*|{>`.
    for i in 0..bytes.len() {
        if matches!(bytes[i], b'+' | b'*')
            && bytes.get(i + 1) == Some(&b')')
            && quant(bytes.get(i + 2))
        {
            return true;
        }
    }
    // Overlapping alternation: `( [^)]* | [^)]* ) <+|*|{>`.
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut has_pipe = false;
            while j < bytes.len() && bytes[j] != b')' {
                has_pipe |= bytes[j] == b'|';
                j += 1;
            }
            if j < bytes.len() && has_pipe && quant(bytes.get(j + 1)) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Destructive builtins whose bare `catch {<cmd> ...}` form is the
/// documented "fire-and-forget" idiom — failure when the target is
/// already gone is expected and intentionally ignored.
fn fire_and_forget_bare(bare: &str) -> bool {
    matches!(bare, "close" | "unset" | "rename")
}

/// Ensemble commands where only certain destructive subcommands are
/// fire-and-forget (`chan close` is, `chan configure` is not).
fn fire_and_forget_subcommand(bare: &str, sub: &str) -> bool {
    match bare {
        "after" => sub == "cancel",
        "chan" => sub == "close",
        "array" | "dict" => sub == "unset",
        "interp" | "file" => sub == "delete",
        "namespace" => sub == "delete" || sub == "forget",
        _ => false,
    }
}

/// True when the body of a `catch` matches the documented
/// "fire-and-forget" idiom: a single command whose head is a
/// destructive builtin (`close $h`, `unset var`, `rename foo ""`) or a
/// documented destructive ensemble subcommand (`after cancel`, `chan
/// close`, `array unset`, …).  Conservative: only single-statement
/// bodies match, and ensemble heads are subcommand-checked.
fn catch_body_is_fire_and_forget(body: &str) -> bool {
    let segs: Vec<_> = crate::segmenter::segment_commands(body)
        .into_iter()
        .filter(|c| !c.texts.is_empty())
        .collect();
    if segs.len() != 1 {
        return false;
    }
    let Some(head) = segs[0].texts.first() else {
        return false;
    };
    if head.is_empty() {
        return false;
    }
    let bare = head
        .trim_start_matches(':')
        .rsplit("::")
        .next()
        .unwrap_or(head);
    if fire_and_forget_bare(bare) {
        return true;
    }
    match segs[0].texts.get(1) {
        Some(first_arg) => fire_and_forget_subcommand(bare, first_arg),
        None => false,
    }
}

/// One W127 closed-value check: return a `(message, span)` hit when the literal
/// value at command-level index `cmd_idx` is not among `allowed` — an exact
/// match, or (when `accept_prefix`) a unique prefix, mirroring C Tcl's
/// abbreviation rule for `string is <class>`. Dynamic values (`$`/`[`) and
/// declared option flags are skipped, and an empty allowed set never fires.
fn w127_closed_hit(
    value: &str,
    span: tcl_lexer::Span,
    allowed: &[&str],
    accept_prefix: bool,
    opt_names: &std::collections::HashSet<&str>,
    display_name: &str,
) -> Option<(String, tcl_lexer::Span)> {
    if allowed.is_empty() || value.contains('$') || value.contains('[') || opt_names.contains(value)
    {
        return None;
    }
    let valid = if accept_prefix {
        // C Tcl's abbreviation rule (`Tcl_GetIndexFromObj`): an exact match
        // always wins; otherwise the value must be a *unique* prefix — an
        // abbreviation matching exactly one allowed value. An ambiguous prefix
        // (matching two or more, e.g. `a` → alnum/alpha/ascii) is a runtime
        // error, so it must NOT be treated as valid.
        allowed.contains(&value)
            || (!value.is_empty() && allowed.iter().filter(|a| a.starts_with(value)).count() == 1)
    } else {
        allowed.contains(&value)
    };
    if valid {
        return None;
    }
    let allowed_list = allowed.join(", ");
    Some((
        format!("Invalid value '{value}' for '{display_name}'; expected one of: {allowed_list}"),
        span,
    ))
}

/// True when the **source** slice `raw` (backslashes intact) carries a
/// *live* substitution: an unescaped `[`, or a `$` that actually
/// introduces a variable name (`[A-Za-z0-9_]`, `{`, or `:`).  A `\[` /
/// `\$` is a literal regex character, and a `$` before a quote / end /
/// punctuation (the `(.*)$` end-anchor) is a literal dollar — neither
/// counts.
fn raw_has_live_substitution(raw: &str) -> bool {
    let b = raw.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        match b[i] {
            b'\\' => {
                i += 2; // the next char is escaped (literal) — skip both
                continue;
            }
            b'[' => return true,
            b'$' => {
                if let Some(&c) = b.get(i + 1)
                    && (c.is_ascii_alphanumeric() || matches!(c, b'_' | b'{' | b':'))
                {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Credential-bearing option flags whose literal values trip W310.
const DEFAULT_PASSWORD_OPTIONS: [&str; 5] = ["-password", "-pass", "-secret", "-token", "-apikey"];

/// True when `value` is a literal (not a `$var` / `[cmd]` substitution)
/// — the W310 literal-value gate.
fn is_literal_credential_value(value: &str, tok: &tcl_lexer::Token) -> bool {
    !matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) && !value.starts_with('$')
        && !value.contains('[')
}

/// Return `true` when an `uplevel` first argument is a level
/// specifier (`1`, `#0`, …) rather than the script itself: strip
/// any leading `#` then require a non-empty all-digit remainder.
fn uplevel_has_level(arg0: &str) -> bool {
    let stripped = arg0.trim_start_matches('#');
    !stripped.is_empty() && stripped.bytes().all(|b| b.is_ascii_digit())
}

/// Parse `subst`'s flags, returning `(template_idx, nocommands,
/// novariables)` — the index of the first non-option argument (the
/// template) and which substitution-suppressing flags were seen.
/// (`-nobackslashes` is accepted but irrelevant to the W102 message.)
fn parse_subst_flags(args: &[String]) -> (Option<usize>, bool, bool) {
    let mut nocommands = false;
    let mut novariables = false;
    let mut template_idx = None;
    for (i, text) in args.iter().enumerate() {
        match text.as_str() {
            "-nocommands" => nocommands = true,
            "-novariables" => novariables = true,
            "-nobackslashes" => {}
            t if t.starts_with('-') => {}
            _ => {
                template_idx = Some(i);
                break;
            }
        }
    }
    (template_idx, nocommands, novariables)
}

/// Return `(pattern_text, token)` pairs for every regex pattern
/// argument in a `regexp` / `regsub` / `switch -regexp` command.
/// `regexp` / `regsub` contribute
/// their first positional (option-skipping) argument; `switch -regexp`
/// contributes every non-`default` pattern arm — inline pairs (form 1)
/// or a single braced case list (form 2, re-segmented via
/// [`super::handlers::parse_switch_body_elements`]).
fn find_regex_patterns_in_command(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
) -> Vec<(String, tcl_lexer::Token)> {
    if args.is_empty() || arg_tokens.is_empty() {
        return Vec::new();
    }
    match cmd_name {
        "regexp" | "regsub" => {
            // Skip leading flags to the pattern argument (`-start`
            // consumes its value; `--` ends the option section).
            let mut idx = 0;
            while idx < args.len() && args[idx].starts_with('-') && args[idx] != "--" {
                if args[idx] == "-start" && idx + 1 < args.len() {
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            match (args.get(idx), arg_tokens.get(idx)) {
                (Some(text), Some(tok)) => vec![(text.clone(), *tok)],
                _ => Vec::new(),
            }
        }
        "switch" => {
            let mut is_regexp = false;
            let mut i = 0;
            while i < args.len() && args[i].starts_with('-') {
                if args[i] == "-regexp" {
                    is_regexp = true;
                }
                if args[i] == "--" {
                    i += 1;
                    break;
                }
                i += 1;
            }
            if !is_regexp {
                return Vec::new();
            }
            // Skip the `string` argument.
            i += 1;
            let mut results = Vec::new();
            if i < args.len() && i == args.len() - 1 {
                // Form 2: single braced case list.
                if let Some(case_tok) = arg_tokens.get(i) {
                    let elements = super::handlers::parse_switch_body_elements(&args[i], *case_tok);
                    let mut j = 0;
                    while j + 1 < elements.len() {
                        let (text, tok) = &elements[j];
                        if text != "default" {
                            results.push((text.clone(), *tok));
                        }
                        j += 2;
                    }
                }
            } else {
                // Form 1: inline pattern/body pairs.
                while i + 1 < args.len() {
                    if let (Some(text), Some(tok)) = (args.get(i), arg_tokens.get(i))
                        && text != "default"
                    {
                        results.push((text.clone(), *tok));
                    }
                    i += 2;
                }
            }
            results
        }
        _ => Vec::new(),
    }
}
