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
use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;

impl Analyser {
    /// Registry spec for `cmd_name` (`None` when no registry is loaded or
    /// the command is unknown).  Shared lookup for the trait-gated security
    /// checks below — none of them match on command-name strings.
    fn security_spec(&self, cmd_name: &str) -> Option<&tcl_registry::CommandSpec> {
        self.registry.as_deref().and_then(|r| r.get(cmd_name))
    }

    /// W101's gate: a command that concatenates **all** of its arguments
    /// into a script and re-parses the result (`eval`).
    ///
    /// [`Traits::EVALUATES_CODE`] alone is too wide for this shape:
    /// the coroutine injectors (`coroinject` / `coroprobe`) carry it but
    /// take a coroutine name plus a command *prefix* — a word list that is
    /// never re-parsed.  The concat-reparse shape is
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`], the registry's own record of
    /// "every trailing word joins into one re-parsed script", paired with
    /// [`Traits::TAINT_SINK`].
    ///
    /// `uplevel` is excluded by [`Traits::EVALUATES_IN_SHIFTED_FRAME`], which
    /// is *why* it belongs to W301 instead: injecting into a script that runs
    /// in the caller's frame is a frame-escalation finding with its own
    /// message, not the same warning as injecting into a same-frame `eval`.
    ///
    /// The gate used to read that split *indirectly* — "no
    /// `arg_role_resolver`, with a fixed [`ArgRole::Body`] at argument 0" —
    /// which made W101's scope an accident of how a spec spells its argument
    /// layout rather than of what the command does. Both halves now name the
    /// semantics, so re-modelling `eval`'s roles cannot silently switch W101
    /// off (issue #1051).
    fn is_concat_eval_command(&self, cmd_name: &str) -> bool {
        self.security_spec(cmd_name).is_some_and(|s| {
            s.traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS | Traits::TAINT_SINK)
                && !s.traits.contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
        })
    }

    /// W301's gate: a concat-reparse taint sink whose script runs in a
    /// **different stack frame** than the call is written in — `uplevel`.
    /// The exact complement of [`Self::is_concat_eval_command`] within the
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`] + [`Traits::TAINT_SINK`] pair, so
    /// every family member is owned by exactly one of the two codes and
    /// neither double-reports.
    fn is_level_eval_command(&self, cmd_name: &str) -> bool {
        self.security_spec(cmd_name).is_some_and(|s| {
            s.traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS | Traits::TAINT_SINK)
                && s.traits.contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
        })
    }

    /// W309's gate: any command that re-parses argument text as a script
    /// and absorbs tainted data — [`Traits::EVALUATES_CODE`] +
    /// [`Traits::TAINT_SINK`] (`eval`, `uplevel`).  The coroutine
    /// injectors carry only the former (command-prefix words, no
    /// re-parse) and stay out.
    fn is_script_reparse_sink(&self, cmd_name: &str) -> bool {
        self.security_spec(cmd_name).is_some_and(|s| {
            s.traits
                .contains(Traits::EVALUATES_CODE | Traits::TAINT_SINK)
        })
    }

    /// True when the inner script of a `[…]` substitution invokes a
    /// substitution performer ([`Traits::PERFORMS_SUBSTITUTION`] — `subst`)
    /// as its command head.  Registry-driven replacement for the former
    /// literal `subst` prefix match; `get` resolves a leading `::`, so the
    /// fully-qualified `[::subst …]` spelling is caught too.
    fn inner_head_performs_substitution(&self, inner: &str) -> bool {
        let head = inner
            .split(|c: char| c.is_ascii_whitespace())
            .next()
            .unwrap_or("");
        !head.is_empty()
            && self
                .security_spec(head)
                .is_some_and(|s| s.traits.contains(Traits::PERFORMS_SUBSTITUTION))
    }

    /// **W302.** Emit "catch without result variable" hint when a
    /// `catch BODY` invocation omits the optional `RESULTVAR`
    /// argument, silently swallowing any error the body raises.
    ///
    /// W302 is emitted for `Statement::Catch` (not `Statement::Barrier`)
    /// — the lowerer falls back to `Statement::Barrier` when the body
    /// argument is multi-token (e.g. ``catch $body``), so this emit gates
    /// on ``arg_single[0]`` to suppress that case.  The diagnostic
    /// anchors at just the ``catch`` command token — the narrowest span
    /// that identifies the issue.
    ///
    /// The attached quick-fixes carry their **own** span, which is *not*
    /// the diagnostic's: they insert at the point past the last supplied
    /// argument's closing delimiter, computed by
    /// [`Self::trailing_arg_fixes`] from the argument tokens.  A consumer
    /// that instead reconstructed an insertion point from the diagnostic's
    /// end would write the new word between `catch` and its body and
    /// silently shift every argument one position along — the corruption
    /// issue #1190 reports.
    pub(in crate::analyser) fn emit_w302_catch_no_result_var(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        // Only fires when a result variable is absent.  Empty args
        // is a malformed catch (Statement::Barrier path, no W302).
        // ≥2 args means a result variable is present.
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
            && catch_body_is_fire_and_forget(body, self.registry.as_deref())
        {
            return;
        }
        let fixes = self.trailing_arg_fixes(cmd_name, args, arg_tokens, "Add catch");
        let span = cmd_tok.span;
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W302,
            span,
            message: "catch without a result variable silently swallows errors. \
Consider capturing the result: catch {\u{2026}} result"
                .to_string(),
            severity: Severity::Hint,
            fixes,
        });
    }

    /// Build the "append the omitted optional result variable(s)" quick-fixes
    /// for an invocation that left declared trailing
    /// [`ArgRole::VarWrite`] slots unfilled.
    ///
    /// Every part of this is registry data, so it holds for any command with
    /// the shape, not for `catch` alone:
    ///
    /// * **How many words may be appended, and what they are called** comes
    ///   from [`tcl_registry::CommandRegistry::unfilled_trailing_roles`] (the
    ///   declared roles beyond the supplied arguments) intersected with
    ///   [`tcl_registry::CommandSpec::optional_trailing_arg_names`] (the
    ///   dialect-applicable synopsis placeholders).  Under a dialect whose
    ///   form documents fewer optional words — Tcl 8.4 and iRules give
    ///   `catch` a result variable but no options dictionary — only the
    ///   narrower fix is offered.
    /// * **Where the words go** is the append point past the *last supplied
    ///   argument*, from [`tcl_lexer::word_append_offset`], so a braced,
    ///   multi-line, quoted, empty, or bracketed body all anchor correctly.
    ///
    /// One fix per prefix of the run, so a caller sees "append the result
    /// variable" and "append the result and options variables" as separate
    /// offers.  Each is a zero-width insertion, and each is classified
    /// [`FixSafety::BehaviourHardening`]: capturing the result stops errors
    /// being swallowed, but it also *writes a new variable in the caller's
    /// frame*, which a program that already uses that name would observe.
    fn trailing_arg_fixes(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        title_prefix: &str,
    ) -> Vec<super::types::CodeFix> {
        let Some(registry) = self.registry.as_deref() else {
            return Vec::new();
        };
        let Some(spec) = registry.get(cmd_name) else {
            return Vec::new();
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // The declared roles of the words a caller could still append …
        let unfilled = registry.unfilled_trailing_roles(cmd_name, &arg_strs);
        // … restricted to the leading run that writes variables: a slot
        // taking a value rather than a variable name is not a place to
        // splice a capture variable.
        let writable = unfilled
            .iter()
            .take_while(|(_, role)| *role == ArgRole::VarWrite)
            .count();
        // … and to the optional words this dialect's synopsis documents.
        // The floor is `None` (permissive) because this runs *during* the
        // walk: `package require` may appear anywhere in the file, so the
        // owning package's resolved version is not known until the post-walk
        // flush, by which time the fix has already been attached.
        let documented = spec.optional_trailing_arg_names(self.profile.availability_mask, None);
        let offered = writable.min(documented.len());
        if offered == 0 {
            return Vec::new();
        }
        let Some(&last) = arg_tokens.last() else {
            return Vec::new();
        };
        let source_map = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let insert_at = tcl_lexer::word_append_offset(&source_map, last);
        // Defensive: an insertion point outside the buffer (a truncated or
        // otherwise malformed document) is not a usable edit.
        if insert_at as usize > self.source.len() {
            return Vec::new();
        }
        let span = tcl_lexer::Span::new(insert_at, insert_at);
        (1..=offered)
            .map(|count| {
                let names: Vec<String> = documented[..count]
                    .iter()
                    .map(|placeholder| variable_name_for_placeholder(placeholder))
                    .collect();
                super::types::CodeFix {
                    span,
                    new_text: format!(" {}", names.join(" ")),
                    description: format!("{title_prefix} {} variable(s)", names.join(" + ")),
                    safety: super::types::FixSafety::BehaviourHardening,
                }
            })
            .collect()
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
        if !self.is_concat_eval_command(cmd_name) || args.is_empty() || arg_tokens.is_empty() {
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
            message: format!(
                "{cmd_name} with substituted arguments risks code injection. \
Prefer direct invocation or {{*}}$cmdList to preserve argument boundaries."
            ),
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
            // W101: `eval [list …]` deliberately stops the substituted words
            // being re-parsed as script — the injection fix, and a change for a
            // caller that meant them to be.
            safety: crate::irules_checks::FixSafety::BehaviourHardening,
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
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        let Some(script) = Analyser::source_slice(&self.source, start, end).map(str::trim) else {
            return false;
        };
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
    /// token or an unusable span.
    fn cmd_token_inner(&self, tok: tcl_lexer::Token) -> Option<&str> {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return None;
        }
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end {
            return None;
        }
        Analyser::source_slice(&self.source, start, end).map(str::trim)
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
        // Registry gate: [`Traits::SOURCES_FILE`] marks the file-executing
        // command (`source`) — the diagnostic applies to any spec that
        // reads and evaluates a script file named by its argument.
        if args.is_empty()
            || arg_tokens.is_empty()
            || !self
                .security_spec(cmd_name)
                .is_some_and(|s| s.traits.contains(Traits::SOURCES_FILE))
        {
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
            // Whole-file `$var` literal resolution — deferred to the tail on
            // the per-item path, exactly as in `emit_w103_open_pipeline`.
            if self.capture_global_reads.is_some() {
                self.pending_var_literal_checks
                    .push((DiagCode::W300, cmd_name.to_string(), tok));
                return;
            }
            self.emit_w300_dynamic_path(cmd_name, tok);
        }
    }

    /// The `$var` / `[cmd]` half of [`Self::emit_w300_source_variable`]'s
    /// classification, split out so the per-item tail can run it once the
    /// whole file is in `self.source` (see
    /// [`super::super::state::Analyser::pending_var_literal_checks`]).
    pub(in crate::analyser) fn emit_w300_dynamic_path(
        &mut self,
        cmd_name: &str,
        tok: tcl_lexer::Token,
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
            message: format!(
                "{cmd_name} with a dynamic path (variable or command substitution) \
executes arbitrary Tcl code. Ensure the path is not influenced by untrusted input."
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
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
        if !self.is_script_reparse_sink(cmd_name) || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        for tok in arg_tokens {
            let Some(inner) = self.cmd_token_inner(*tok) else {
                continue;
            };
            if self.inner_head_performs_substitution(inner) {
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
        if !self.is_level_eval_command(cmd_name) || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // The literal-level probe stays local rather than reusing the
        // spec's arg-role resolver: the resolver treats a *dynamic* first
        // word (`uplevel $lvl $body`) as a level when a script word
        // follows, but for injection purposes a substituted word must be
        // scanned as script — the conservative posture this check has
        // always taken.
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
                    message: format!(
                        "{cmd_name} with multiple arguments concatenates them into \
a script (like eval). Use a single braced body or {{*}}$cmdList to avoid injection."
                    ),
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
                    message: format!(
                        "{cmd_name} with an unbraced script argument may cause \
double substitution. Use braces: {cmd_name} 1 {{...}}"
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W312.** Emit "interp eval / invokehidden injection" when a
    /// cross-interpreter script argument risks injection — the same shape
    /// as W301 but for the child-interpreter dispatch.
    ///
    /// The eligible subcommands are the parent spec's
    /// `taint_interp_eval_subcommands` list (`interp eval` /
    /// `interp invokehidden`, Tk's `console eval` and
    /// `consoleinterp eval|record`) — never a command-name match.  The
    /// subcommand word resolves through the registry's ensemble rule
    /// (exact or unique prefix, C Tcl's `Tcl_GetIndexFromObj`), so the
    /// abbreviated `interp ev …` C Tcl accepts is checked too.
    pub(in crate::analyser) fn emit_w312_interp_eval_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        let Some((sub_name, script_start, concatenates)) =
            self.interp_eval_script_location(cmd_name, args)
        else {
            return;
        };
        if script_start >= args.len() || script_start >= arg_tokens.len() {
            return;
        }
        let script_args = &args[script_start..];
        let script_toks = &arg_tokens[script_start..];
        // A Body-role sink (`interp eval`) with multiple script words
        // concatenates them, like `eval`.
        if concatenates && script_args.len() > 1 {
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W312,
                    span: script_toks[0].span,
                    message: format!(
                        "{cmd_name} {sub_name} with multiple arguments concatenates \
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
                    "{cmd_name} {sub_name} with an unbraced script argument may \
cause code injection. Use braces: {cmd_name} {sub_name} $child {{...}}"
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// Resolve a W312 call site against the registry: `Some((canonical
    /// subcommand name, absolute index of the first script word, whether
    /// multiple script words concatenate))` when `cmd_name`'s spec lists
    /// the resolved subcommand in `taint_interp_eval_subcommands`, else
    /// `None`.
    ///
    /// The script position comes from the subcommand's declared
    /// [`ArgRole::Body`] (`interp eval PATH script` → absolute index 2;
    /// `console eval script` → 1); a listed sink *without* a Body role
    /// (`interp invokehidden` — the words are invoked verbatim, never
    /// re-parsed) locates the hidden command word instead: past the
    /// interpreter path, skipping `-opt` words.
    fn interp_eval_script_location(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<(&'static str, usize, bool)> {
        let spec = self.security_spec(cmd_name)?;
        if spec.taint_interp_eval_subcommands.is_empty() || args.is_empty() {
            return None;
        }
        let sub = spec.resolve_subcommand(&args[0])?;
        if !spec.taint_interp_eval_subcommands.contains(&sub.name) {
            return None;
        }
        if let Some((idx, _)) = sub.arg_roles.iter().find(|(_, r)| *r == ArgRole::Body) {
            // +1: subcommand arg roles are relative to the word after the
            // subcommand.
            return Some((sub.name, *idx as usize + 1, true));
        }
        let mut i = 2;
        while i < args.len() && args[i].starts_with('-') {
            i += 1;
        }
        (i < args.len()).then_some((sub.name, i, false))
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
        // Registry gate: [`Traits::PERFORMS_SUBSTITUTION`] marks the
        // template-expanding command (`subst`) — any spec that performs
        // `$var` / `[cmd]` substitution over an argument string.
        if args.is_empty()
            || arg_tokens.is_empty()
            || !self
                .security_spec(cmd_name)
                .is_some_and(|s| s.traits.contains(Traits::PERFORMS_SUBSTITUTION))
        {
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
            "{cmd_name} with a variable argument enables code injection: any \
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
        // Registry gate: [`Traits::OPENS_CHANNEL`] marks the channel
        // factories (`open`, `socket`).  The pipeline interpretation of a
        // leading `|` in the first argument belongs to the file-open family
        // only (`Tcl_OpenObjCmd` → `TclOpenCommandChannel`, tclIOCmd.c); a
        // network socket's first argument is an address / `-option`, never
        // an exec spec, and its spec carries [`Traits::TAINT_SOURCE`]
        // (remote-peer data) which the local file `open` deliberately does
        // not — so the taint trait is the behaviour-preserving excluder for
        // the non-pipeline channel openers.
        if args.is_empty()
            || arg_tokens.is_empty()
            || !self.security_spec(cmd_name).is_some_and(|s| {
                s.traits.contains(Traits::OPENS_CHANNEL) && !s.traits.contains(Traits::TAINT_SOURCE)
            })
        {
            return;
        }
        let tok = arg_tokens[0];
        if args[0].starts_with('|') {
            let (severity, message) = if self.args_have_substitution(arg_tokens, arg_single) {
                (
                    Severity::Warning,
                    format!(
                        "{cmd_name} with a pipeline containing variable/command \
substitution risks command injection. Validate and sanitize the command \
before passing to {cmd_name}."
                    ),
                )
            } else {
                (
                    Severity::Hint,
                    format!(
                        "{cmd_name} with a pipeline (\"|\") executes an external command. \
Ensure the command is not influenced by untrusted input."
                    ),
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
            // The classification below resolves `$var` against the whole
            // file's most recent literal `set` — a whole-file fact an
            // isolated proc body cannot answer (its `self.source` is only
            // the body), so the per-item path defers it to the tail, like
            // `pending_w304`.  See `Analyser::pending_var_literal_checks`.
            if self.capture_global_reads.is_some() {
                self.pending_var_literal_checks
                    .push((DiagCode::W103, cmd_name.to_string(), tok));
                return;
            }
            self.emit_w103_dynamic_first_arg(cmd_name, tok);
        }
    }

    /// The `$var` / `[cmd]` half of [`Self::emit_w103_open_pipeline`]'s
    /// classification, split out so the per-item tail can run it once the
    /// whole file is in `self.source` (see
    /// [`super::super::state::Analyser::pending_var_literal_checks`]).
    pub(in crate::analyser) fn emit_w103_dynamic_first_arg(
        &mut self,
        cmd_name: &str,
        tok: tcl_lexer::Token,
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
                    message: format!(
                        "{cmd_name} with a pipeline (\"|\") executes an external command. \
Ensure the command is not influenced by untrusted input."
                    ),
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
            message: format!(
                "{cmd_name} with a dynamic argument (variable or command substitution): \
if the value starts with \"|\", it will execute a command pipeline. Validate input \
or use explicit I/O commands."
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
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
        let takes_regex_pattern = self.command_takes_regex_pattern(cmd_name);
        // `switch` stays name-guarded: its patterns are glob by default and
        // regex only under `-regexp`, so its spec is not `PatternType::Regex`;
        // the arm scan below is switch-form-specific.
        if !takes_regex_pattern && cmd_name != "switch" {
            return;
        }
        let patterns = find_regex_patterns_in_command(
            &self.source,
            takes_regex_pattern,
            cmd_name,
            args,
            arg_tokens,
        );
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
        if !self.command_takes_regex_pattern(cmd_name) {
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
        if start >= end {
            return;
        }
        let Some(raw) = Analyser::source_slice(&self.source, start, end) else {
            return;
        };
        let is_subst_token = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        // A quoted/literal token (not a single `$var` / `[cmd]` word) only
        // counts when the raw source carries a *live* (unescaped) `[`/`$`.
        if !is_subst_token && !raw_has_live_substitution(raw) {
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
        let mut hits: Vec<W127Hit> = Vec::new();
        if let Some(registry) = self.registry.clone() {
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
                let values = spec.arg_values_at(idx);
                let allowed: Vec<&str> = values.iter().map(|av| av.value).collect();
                if let Some(hit) =
                    w127_closed_hit(value, span_at(i), &allowed, false, &opt_names, cmd_name)
                {
                    hits.push(hit);
                } else {
                    self.record_w137_gated_value(value, span_at(i), values, false, cmd_name);
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
                    } else {
                        self.record_w137_gated_value(
                            value,
                            span_at(i),
                            sub.arg_values_at(sub_idx),
                            sub.arg_values_accept_prefix,
                            &display,
                        );
                    }
                }
            }
        }
        for hit in hits {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W127,
                span: hit.span,
                message: hit.message,
                severity: Severity::Warning,
                fixes: hit.fixes,
            });
        }
    }

    /// Buffer a W137 site when a **valid** closed argument value is
    /// version-gated ([`ArgValue::min_tcl`], the §6 argument-DSL rung):
    /// `string is dict` names a real class — on Tcl 9.0; below it the
    /// class itself raises at runtime. Decided post-walk against the
    /// effective Tcl version, like every version fact.
    ///
    /// [`ArgValue::min_tcl`]: tcl_registry::ArgValue
    fn record_w137_gated_value(
        &mut self,
        value: &str,
        span: tcl_lexer::Span,
        values: &'static [tcl_registry::ArgValue],
        accept_prefix: bool,
        display_name: &str,
    ) {
        if value.contains('$') || value.contains('[') {
            return;
        }
        let matched = values.iter().find(|av| av.value == value).or_else(|| {
            if !accept_prefix || value.is_empty() {
                return None;
            }
            // C Tcl's abbreviation rule: a unique prefix selects the value.
            let mut it = values.iter().filter(|av| av.value.starts_with(value));
            match (it.next(), it.next()) {
                (Some(av), None) => Some(av),
                _ => None,
            }
        });
        if let Some(av) = matched
            && let Some(min) = av.min_tcl
        {
            self.dsl_gate_sites.push(super::version_gate::DslGateSite {
                span,
                code: DiagCode::W137,
                what: format!("argument value '{}' of '{display_name}'", av.value),
                min,
            });
        }
    }

    /// **W127 (option-value sibling)** and **W141**. A literal value word of
    /// a value-taking option whose value set is *closed*
    /// (`OptionValue::enumerated(.., true, ..)`) is not in that set (W127) —
    /// unless the option also declares an [`tcl_registry::hover::IntegerDomain`]
    /// (`return -code ok|error|...|<int>`) and the value parses as a Tcl
    /// integer within it, which is accepted alongside the closed set rather
    /// than instead of it. Separately, an option whose arity is
    /// [`tcl_registry::hover::OptionArity::Hook`] runs its resolver as a
    /// content check: a `Some(msg)` `invalid` result is flagged as W141 (a
    /// value that's structurally malformed — `-errorstack`'s value must be
    /// an even-sized list — rather than merely outside a closed set).
    /// Options are matched by name or alias, arity and the `--` terminator
    /// are honoured via `value_indices`, and dynamic values (`$var` /
    /// `[cmd]`) are skipped for both checks — mirroring the positional path
    /// without overloading it.
    pub(in crate::analyser) fn emit_w127_closed_option_values(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        cmd_tok: tcl_lexer::Token,
    ) {
        let mut hits: Vec<W127Hit> = Vec::new();
        let mut hook_hits: Vec<W127Hit> = Vec::new();
        if let Some(registry) = self.registry.as_deref() {
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
                let occ = OptionOccurrence {
                    cmd_name,
                    arg,
                    opt,
                    vals: &vals,
                    args,
                    arg_tokens,
                    flag_idx: i,
                    cmd_tok,
                };
                hits.extend(w127_domain_hits(&occ, registry));
                hook_hits.extend(w141_hook_hit(&occ));
                i += 1 + vals.len();
            }
        }
        for hit in hits {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W127,
                span: hit.span,
                message: hit.message,
                severity: Severity::Warning,
                fixes: hit.fixes,
            });
        }
        for hit in hook_hits {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W141,
                span: hit.span,
                message: hit.message,
                severity: Severity::Warning,
                fixes: hit.fixes,
            });
        }
    }

    /// **W310.** Emit "hardcoded credential" for a literal secret value.
    /// Two strategies, one diagnostic per command:
    ///
    /// * **Strategy 1** — a credential-bearing option flag (the registry's
    ///   command-independent
    ///   [`tcl_registry::spec::DEFAULT_CREDENTIAL_OPTION_NAMES`],
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
        // the `self.registry.as_deref()` borrow ends with this binding).
        let extra_opts: &'static [&'static str] = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .map_or(&[], |s| s.credential_options);

        // Strategy 1: a credential option flag with a literal value.
        for (i, text) in args.iter().enumerate() {
            let lower = text.to_ascii_lowercase();
            if !tcl_registry::spec::DEFAULT_CREDENTIAL_OPTION_NAMES.contains(&lower.as_str())
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

/// Turn a synopsis placeholder into the variable name a quick-fix inserts.
///
/// Synopsis placeholders name a *role*, in Tcl manual-page style: the word
/// documented as `resultVarName` is the variable that receives the result.
/// Spliced into source verbatim it reads as documentation rather than code,
/// so the trailing `VarName` / `Name` marker is dropped and the leading
/// letter lower-cased — `resultVarName` becomes `result`,
/// `optionsVarName` becomes `options`, and a placeholder carrying no such
/// marker (`varName` alone, once `Name` is dropped, is `var`) keeps its
/// remaining spelling.  A placeholder that would reduce to nothing keeps
/// its original text so the fix always inserts a usable identifier.
fn variable_name_for_placeholder(placeholder: &str) -> String {
    let stem = placeholder
        .strip_suffix("VarName")
        .or_else(|| placeholder.strip_suffix("Name"))
        .filter(|stem| !stem.is_empty())
        .unwrap_or(placeholder);
    let mut chars = stem.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => placeholder.to_string(),
    }
}

/// True when the body of a `catch` matches the documented
/// "fire-and-forget" idiom: a single command whose head is a teardown
/// builtin (`close $h`, `unset var`, `rename foo ""`) or a teardown
/// ensemble subcommand (`after cancel`, `chan close`, `array unset`, …).
/// Membership comes from the registry's
/// [`tcl_registry::Traits::FIRE_AND_FORGET_TEARDOWN`] trait, at both the
/// command and the resolved-subcommand level.  Conservative: only
/// single-statement bodies match, and ensemble heads are
/// subcommand-checked.
fn catch_body_is_fire_and_forget(
    body: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> bool {
    let Some(registry) = registry else {
        return false;
    };
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
    // Resolve a namespace-qualified spelling to its tail, matching the
    // pre-registry behaviour (`::close` and `myns::close` both counted).
    let bare = head
        .trim_start_matches(':')
        .rsplit("::")
        .next()
        .unwrap_or(head);
    let Some(spec) = registry.get(bare) else {
        return false;
    };
    let teardown = tcl_registry::Traits::FIRE_AND_FORGET_TEARDOWN;
    if spec.traits.contains(teardown) {
        return true;
    }
    segs[0]
        .texts
        .get(1)
        .and_then(|first_arg| spec.resolve_subcommand(first_arg))
        .is_some_and(|sub| sub.traits.contains(teardown))
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
) -> Option<W127Hit> {
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
    let mut message =
        format!("Invalid value '{value}' for '{display_name}'; expected one of: {allowed_list}");
    let fixes = w127_suggestion_fix(value, allowed, span, &mut message);
    Some(W127Hit {
        message,
        span,
        fixes,
    })
}

/// One W127 finding: the message, its anchor, and the "did you mean…?"
/// replace fix (empty when no allowed value is close enough).
struct W127Hit {
    message: String,
    span: tcl_lexer::Span,
    fixes: Vec<crate::analyser::types::CodeFix>,
}

/// Append a "; did you mean 'X'?" suffix to `message` and build the
/// replace fix when one allowed value sits within the length-scaled edit
/// budget of `value`.  The allowed set is already in the message, so a
/// missing suggestion loses nothing.
/// One resolved option flag's context within the current command's args —
/// threaded through [`w127_domain_hits`]/[`w141_hook_hit`] instead of as
/// loose parameters, mirroring `commands::DispatchSite`'s bundled-context
/// shape.
struct OptionOccurrence<'a> {
    cmd_name: &'a str,
    arg: &'a str,
    opt: &'a tcl_registry::hover::OptionSpec,
    vals: &'a [usize],
    args: &'a [String],
    arg_tokens: &'a [tcl_lexer::Token],
    flag_idx: usize,
    cmd_tok: tcl_lexer::Token,
}

/// Per-occurrence closed-set / integer-domain check for
/// [`Analyser::emit_w127_closed_option_values`] — one option's value
/// word(s) against its declared `values`/`integer`. Returns one hit per
/// invalid word; empty when the option declares neither (an open value).
fn w127_domain_hits(
    occ: &OptionOccurrence<'_>,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<W127Hit> {
    // Whether a literal is a Tcl integer is release-dependent (`08` and `1_0`
    // are integers from 9.0 and not before), so the domain check reads it under
    // the release being analysed rather than the ambient grammar.
    let numbers = registry.numbers();
    let allowed = occ.opt.value_values();
    let integer_domain = occ.opt.value_integer_domain();
    let has_closed_set = occ.opt.value_is_closed() && !allowed.is_empty();
    if !has_closed_set && integer_domain.is_none() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for &vi in occ.vals {
        let Some(value) = occ.args.get(vi) else {
            continue;
        };
        if value.contains('$') || value.contains('[') {
            continue;
        }
        let matches_literal = allowed.iter().any(|av| av.value == value.as_str());
        let matches_integer = integer_domain.is_some_and(|dom| {
            match tcl_syntax::number::parse_whole_with(
                value,
                tcl_syntax::number::ParseFlags::for_syntax(numbers),
            ) {
                Some(tcl_syntax::number::Number::Int(n)) => dom.accepts(n),
                // A bignum literal (too large for i64) is always outside a
                // bounded Range/Port, but still a legal Tcl integer for `Any`.
                Some(tcl_syntax::number::Number::Big { .. }) => {
                    matches!(dom, tcl_registry::hover::IntegerDomain::Any)
                }
                // A float/NaN literal is never a Tcl integer.
                Some(
                    tcl_syntax::number::Number::Double(_) | tcl_syntax::number::Number::Nan { .. },
                )
                | None => false,
            }
        });
        if matches_literal || matches_integer {
            continue;
        }
        let span = occ.arg_tokens.get(vi).map_or(occ.cmd_tok.span, |t| t.span);
        let (message, fixes) = if has_closed_set {
            let allowed_names: Vec<&str> = allowed.iter().map(|av| av.value).collect();
            let mut allowed_list = allowed_names.join(", ");
            if let Some(dom) = integer_domain {
                allowed_list.push_str(", or ");
                allowed_list.push_str(&describe_integer_domain(dom));
            }
            let mut message = format!(
                "Invalid value '{value}' for option '{}' on '{}'; expected one of: {allowed_list}",
                occ.arg, occ.cmd_name
            );
            let fixes = w127_suggestion_fix(value, &allowed_names, span, &mut message);
            (message, fixes)
        } else {
            // Pure numeric domain (`-level`): nothing in a literal set to
            // suggest against.
            let domain = integer_domain.expect("guarded by the outer `if`");
            (
                format!(
                    "Invalid value '{value}' for option '{}' on '{}'; expected {}",
                    occ.arg,
                    occ.cmd_name,
                    describe_integer_domain(domain)
                ),
                Vec::new(),
            )
        };
        hits.push(W127Hit {
            message,
            span,
            fixes,
        });
    }
    hits
}

/// Per-occurrence [`tcl_registry::hover::OptionArity::Hook`] content check
/// for [`Analyser::emit_w127_closed_option_values`] — `None` when the
/// option's arity isn't `Hook`, the value is dynamic (`$var`/`[cmd]`), or
/// the hook reports the value valid.
fn w141_hook_hit(occ: &OptionOccurrence<'_>) -> Option<W127Hit> {
    let hook = occ.opt.value_arity_hook()?;
    let dynamic = occ
        .vals
        .iter()
        .filter_map(|&vi| occ.args.get(vi))
        .any(|v| v.contains('$') || v.contains('['));
    if dynamic {
        return None;
    }
    let full: Vec<&str> = occ.args.iter().map(String::as_str).collect();
    let msg = hook(&full, occ.flag_idx + 1).invalid?;
    let span = occ
        .vals
        .first()
        .and_then(|&vi| occ.arg_tokens.get(vi))
        .map_or(occ.cmd_tok.span, |t| t.span);
    Some(W127Hit {
        message: format!("{msg} (option '{}' on '{}')", occ.arg, occ.cmd_name),
        span,
        fixes: Vec::new(),
    })
}

/// Human-readable description of an [`tcl_registry::hover::IntegerDomain`]
/// for a W127 message — `describe_integer_domain(Range(0, 2147483647))` →
/// `"an integer between 0 and 2147483647"`.
fn describe_integer_domain(domain: tcl_registry::hover::IntegerDomain) -> String {
    use tcl_registry::hover::IntegerDomain;
    match domain {
        IntegerDomain::Any => "an integer".to_string(),
        IntegerDomain::Range(lo, hi) => format!("an integer between {lo} and {hi}"),
        IntegerDomain::Port => "a port number (0-65535)".to_string(),
    }
}

fn w127_suggestion_fix(
    value: &str,
    allowed: &[&str],
    span: tcl_lexer::Span,
    message: &mut String,
) -> Vec<crate::analyser::types::CodeFix> {
    use std::fmt::Write as _;
    let suggestions = crate::text::suggest_similar(
        value,
        allowed.iter().copied(),
        1,
        crate::text::scaled_max_distance(value),
    );
    let Some(best) = suggestions.first() else {
        return Vec::new();
    };
    let _ = write!(message, "; did you mean '{best}'?");
    vec![crate::analyser::types::CodeFix {
        span,
        new_text: (*best).to_string(),
        description: format!("Replace with '{best}'"),
        // W127: an edit-distance guess at the intended value.
        safety: crate::irules_checks::FixSafety::RequiresReview,
    }]
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
/// argument in a command.  A `PatternType::Regex` command
/// (`takes_regex_pattern` — `regexp` / `regsub`) contributes
/// its first positional (option-skipping) argument; `switch -regexp`
/// contributes every non-`default` pattern arm — inline pairs (form 1)
/// or a single braced case list (form 2, split with list offsets via
/// [`crate::segmenter::flatten_clause_list_elements`]).
fn find_regex_patterns_in_command(
    source: &str,
    takes_regex_pattern: bool,
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
) -> Vec<(String, tcl_lexer::Token)> {
    if args.is_empty() || arg_tokens.is_empty() {
        return Vec::new();
    }
    if takes_regex_pattern {
        // Skip leading flags to the pattern argument via the one canonical
        // regex-command option-skip.
        let Some(idx) = regexp_pattern_index(args) else {
            return Vec::new();
        };
        return match (args.get(idx), arg_tokens.get(idx)) {
            (Some(text), Some(tok)) => vec![(text.clone(), *tok)],
            _ => Vec::new(),
        };
    }
    match cmd_name {
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
                    let elements =
                        crate::segmenter::flatten_clause_list_elements(source, &args[i], *case_tok);
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
