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

//! Command validity and arity checks emitted during the command walk.
//!
//! These diagnostics decide whether a command invocation is well-formed
//! against the registry and the active dialect: an unknown subcommand
//! (W001), a command disabled in the dialect (W002), an invalid dialect
//! option (W004) or expression operator (W003), wrong argument counts
//! (the arity diagnostics), a malformed `if` (E004), a missing `--` option
//! terminator before a value that looks like an option (W304), an `unset`
//! whose options consume every argument so nothing is unset (W217), and a stub
//! `proc` that shadows a built-in command or `expr` function (W116, W117).
//! The disabled-command, arity, and W304 emitters buffer their candidates
//! and flush them after the walk.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_core_types::DiagCode;
use tcl_registry::Arity;

use super::helpers::{has_substitution, is_ident_continue, is_integer_word};
use crate::analyser::state::Analyser;
use crate::analyser::types::{PendingUserCallArity, Severity};
use crate::expr_ast::{BinOp, ExprNode};

/// The argument words of one command invocation, scoped to the prefix the
/// caller has already consumed: `args` / `arg_tokens` / `arg_expand` are the
/// slices *after* that prefix (the command name for the simple path; the
/// command name + subcommand word for the subcommand path), and `cmd_tok`
/// anchors the diagnostic span.  Bundled to keep [`Analyser::check_simple_arity`]
/// under the argument limit.
struct ArityWords<'a> {
    args: &'a [String],
    arg_tokens: &'a [tcl_lexer::Token],
    arg_expand: &'a [bool],
    cmd_tok: tcl_lexer::Token,
}

/// Positional-argument lower bound + whether any positional word is
/// `{*}`-expanded, starting at `start` (the caller has already classified
/// / skipped everything before it — leading declared option flags for a
/// registry command, nothing for a same-file user call). A `{*}`-expanded
/// word contributes an unknown number of runtime arguments, so once one
/// is seen the count becomes a lower bound only — matches
/// [`Analyser::check_simple_arity`]'s original inline formula exactly;
/// shared here so [`Analyser::queue_user_call_arity_candidate`] doesn't
/// reimplement it.
fn count_positionals(args: &[String], arg_expand: &[bool], start: usize) -> (usize, bool) {
    let expanded = |i: usize| arg_expand.get(i).copied().unwrap_or(false);
    let start = start.min(args.len());
    let any_expand = (start..args.len()).any(expanded);
    let nargs_min = if any_expand {
        (start..args.len()).filter(|&i| !expanded(i)).count()
    } else {
        args.len() - start
    };
    (nargs_min, any_expand)
}

/// Widen a single braced (`Str`) / bracketed (`Cmd`) word token's `end`
/// to cover its own closing delimiter — the per-token equivalent of the
/// segmenter's `widen_word_end` (`crate::segmenter`, private to its own
/// recovery logic, so duplicated here rather than exposed cross-module).
/// Per the lexer's inner-end convention (see `AGENTS.md`, "Word-token
/// closing delimiters"), a non-empty braced/bracketed word's `end()`
/// already sits on the closer byte, one short of covering it; an empty
/// `{}` / `[]` already has `end()` past the closer, so it is left alone
/// (widening it could swallow an unrelated adjacent `}` / `]` from
/// whatever encloses this word — `RUST_ISSUE_527`). Any other token kind
/// (a bareword, `then`/`elseif`/`else`, a `Var`, …) has no closer to
/// widen for.
fn widen_token_end(tok: tcl_lexer::Token, source: &str) -> u32 {
    let closer = match tok.kind {
        tcl_lexer::TokenType::Str => b'}',
        tcl_lexer::TokenType::Cmd => b']',
        _ => return tok.span.end(),
    };
    let end = tok.span.end();
    if tcl_lexer::SourceMap::new(source).token_text(tok).is_empty() {
        return end;
    }
    if source.as_bytes().get(end as usize) == Some(&closer) {
        end + 1
    } else {
        end
    }
}

/// A single word token's full source span, closing delimiter included
/// (see [`widen_token_end`]) — for anchoring a diagnostic tightly on one
/// whole word rather than dropping its last byte.
fn widened_word_span(tok: tcl_lexer::Token, source: &str) -> tcl_lexer::Span {
    tcl_lexer::Span::new(tok.span.start(), widen_token_end(tok, source))
}

/// Compare a resolved [`Arity`] against an observed positional-argument
/// count and build the E002 / E003 diagnostic, or `None` when the count
/// fits. Shared by the registry-command arity path
/// ([`Analyser::check_simple_arity`]), the same-file proc / `TclOO`
/// method / `interp alias` / `rename` arity path
/// ([`Analyser::flush_arity_diagnostics`]), and the `TclOO` method-call
/// arity check ([`super::var_command`]), so all three diagnostics carry
/// identical wording.
pub(super) fn arity_verdict(
    display_name: &str,
    arity: Arity,
    nargs_min: usize,
    positional_any_expand: bool,
    span: tcl_lexer::Span,
) -> Option<crate::analyser::types::Diagnostic> {
    let min = usize::from(arity.min);
    let max = usize::from(arity.max);
    if !positional_any_expand && nargs_min < min {
        Some(crate::analyser::types::Diagnostic {
            code: DiagCode::E002,
            span,
            message: format!(
                "Too few arguments for '{display_name}': expected at least {min}, got {nargs_min}"
            ),
            severity: Severity::Error,
            fixes: Vec::new(),
        })
    } else if !arity.is_unlimited() && nargs_min > max {
        Some(crate::analyser::types::Diagnostic {
            code: DiagCode::E003,
            span,
            message: format!(
                "Too many arguments for '{display_name}': expected at most {max}, got {nargs_min}"
            ),
            severity: Severity::Error,
            fixes: Vec::new(),
        })
    } else {
        None
    }
}

/// Namespace-qualify `cmd_name`'s resolution candidates the Tcl way:
/// current namespace first, then global. Shared by the
/// builtin-shadowing suppression check
/// ([`Analyser::flush_arity_diagnostics`]) and the same-file proc/alias/
/// rename resolution chase
/// ([`Analyser::resolve_indirect_call_target`]), so both walk the
/// identical candidate order.
///
/// A name containing `::` but not starting with it (`inner::p`) is
/// still *relative* — Tcl resolves it against the current namespace
/// before falling back to global (confirmed against tclsh 9.0.4:
/// calling `inner::p` from inside `namespace eval ::ns { … }` reaches
/// `::ns::inner::p`, not `::inner::p`, when both exist). Only a
/// leading `::` is a genuinely absolute name.
fn qualify_candidates(ns: &str, cmd_name: &str) -> Vec<String> {
    if cmd_name.starts_with("::") {
        return vec![cmd_name.to_owned()];
    }
    let global = format!("::{cmd_name}");
    if ns == "::" {
        return vec![global];
    }
    let relative = format!("{ns}::{cmd_name}");
    vec![relative, global]
}

/// Shift a resolved [`Arity`] down by an `interp alias` / `TclOO`
/// `forward`'s prepended-argument count — real Tcl partial application
/// (confirmed against tclsh 9.0.4: `interp alias {} short {} target
/// extra` requires exactly `target`'s arity minus one fewer argument at
/// the `short` call site).
///
/// When the prepended count already exceeds a *bounded* target's own
/// max, the alias/forward is unconditionally broken — every call fails
/// at run time regardless of how many further arguments are supplied
/// (confirmed against tclsh 9.0.4: `proc target {a} {}; interp alias {}
/// bad {} target fixed extra; bad` fails "wrong # args" for zero, one,
/// or two further arguments alike). Saturating both bounds to zero would
/// misrepresent that as "callable with exactly zero arguments" — a
/// silent false negative. Returning an unsatisfiable range (`min > max`)
/// instead guarantees `arity_verdict` flags every call, whatever count
/// it's called with.
pub(super) fn shift_arity(arity: Arity, prepended: u16) -> Arity {
    if !arity.is_unlimited() && prepended > arity.max {
        return Arity::new(1, 0);
    }
    let min = arity.min.saturating_sub(prepended);
    let max = if arity.is_unlimited() {
        Arity::UNLIMITED
    } else {
        arity.max.saturating_sub(prepended)
    };
    Arity::new(min, max)
}

/// Add `extra` to both bounds of `arity` — the inverse of [`shift_arity`].
/// Used for `oo::class create NAME ?args?`'s mandatory leading object-name
/// word: a caller-side *extra* required argument ahead of the
/// constructor's own parameters, the opposite of an alias/forward's
/// baked-in prepended args (which the *target* never sees).
fn bump_arity(arity: Arity, extra: u16) -> Arity {
    let min = arity.min.saturating_add(extra);
    let max = if arity.is_unlimited() {
        Arity::UNLIMITED
    } else {
        arity.max.saturating_add(extra)
    };
    Arity::new(min, max)
}

/// Whether `metaclass` denotes a genuine `TclOO` class. Snit (`snit::type` /
/// `snit::widget` / `::snit::widgetadaptor`) and [incr Tcl] (`itcl::class`)
/// instantiate via `TypeName instanceName ?args?`, never `new`/`create` — a
/// class recorded under one of those metaclasses must not be
/// constructor-arity-checked here.
///
/// Registry-driven, not a hardcoded metaclass-name list: `metaclass` is
/// itself a definer *command* name (`oo::class`, `snit::type`, `itcl::class`,
/// …), each registered with a [`DefinitionBodyGrammar`](tcl_registry::definer::DefinitionBodyGrammar)
/// tagged by [`DefinerFamily`](tcl_registry::definer::DefinerFamily), so any
/// future `TclOo`-family metaclass the registry gains is recognised
/// automatically. Shared with `var_command.rs`'s `e001_for_bare_object_dispatch`
/// (the E001 "`$obj` with no method word" check), so the two paths that both
/// need to tell `TclOO` apart from snit/itcl never disagree on the same input.
pub(super) fn is_tcloo_metaclass(
    registry: Option<&tcl_registry::CommandRegistry>,
    metaclass: &str,
) -> bool {
    registry.and_then(|r| r.get(metaclass)).is_some_and(|s| {
        s.definition_body
            .is_some_and(|g| g.family == tcl_registry::definer::DefinerFamily::TclOo)
    })
}

impl Analyser {
    /// **W001.** Emit "Unknown subcommand" warning for commands
    /// whose registry signature is a [`SubcommandSig`](super::dispatch::SubcommandSig)
    /// when the first argument doesn't resolve to a known subcommand.
    ///
    /// Skips:
    ///
    /// - commands the registry doesn't know (no signature),
    /// - simple-command signatures (no subcommand dispatch),
    /// - signatures with `allow_unknown == true` (generated
    ///   dialect packs),
    /// - first-arg values containing ``$`` / ``[`` (dynamic
    ///   substitution — runtime-resolved),
    /// - empty arg lists (handled by the E001 emitter).
    ///
    /// When emission is warranted, includes a "did you mean…?"
    /// suffix using [`crate::text::suggest_similar`] over the
    /// known subcommand set (max 1 suggestion within edit
    /// distance 3).
    ///
    /// One case is not handled: a subcommand position that is
    /// ``{*}``-expanded (``arg_expand[0]``). ``process_command`` does
    /// not currently thread the expansion flag through; the literal-
    /// text ``$`` / ``[`` gate covers the dynamic-substitution case,
    /// and ``{*}LITERAL`` for an unknown subcommand is rare enough in
    /// practice that the gap is acceptable.
    /// **W002** — the command is disabled in the active dialect profile: it
    /// exists in the registry but not for the active dialect (e.g. `dict` under
    /// `tcl8.4`, added in 8.5).  Only a *literal* command head is checked — a
    /// `$obj` / `[cmd]` head is W307's concern — and an earlier unconditional
    /// user-proc definition that shadows the built-in suppresses it (Tcl
    /// resolves the proc at the call site).
    pub(in crate::analyser) fn emit_w002_disabled_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        use tcl_registry::prelude::DialectSet;
        // A dynamic command head (`$obj method`, `[lookup] arg`) is resolved at
        // runtime — W307 handles it, not W002.
        if matches!(
            cmd_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) {
            return;
        }
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let bare = cmd_name.trim_start_matches(':');
        if bare.is_empty() {
            return;
        }
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        // EXISTS in the active dialect → fine.  UNKNOWN everywhere → W123's
        // concern.  Only DISALLOWED (exists in some dialect, not this one)
        // fires.  Existence must be checked *dialect-agnostically*: the
        // analyser registry only loads the active dialect, so `get(bare)`
        // misses an iRules command like `when`/`log`/`session` under
        // tcl8.6, so use the dialect-independent `known_in_any_dialect`.
        if registry.get_for_dialect(bare, dialect).is_some() || !registry.known_in_any_dialect(bare)
        {
            return;
        }
        // An earlier *unconditional* user proc with this name shadows the
        // would-be-disabled built-in at the call site.
        let qualified = crate::naming::normalise_qualified_name(bare);
        if let Some(def) = self.result.all_procs.get(&qualified)
            && def.name_span.start() < cmd_tok.span.start()
        {
            return;
        }
        let diag = super::types::Diagnostic {
            code: DiagCode::W002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is disabled in the active dialect profile"),
            severity: Severity::Warning,
            fixes: Vec::new(),
        };
        // Per-item path (isolated body): the body's own `all_procs` couldn't
        // prove a shadow, but a *sibling/enclosing* user proc still might.  That
        // is a cross-item fact, so defer the shadow re-check to the tail (over
        // the merged `all_procs`).  `capture_global_reads.is_some()` marks the
        // isolated-body analysis; on the whole-file path it is `None` and W002 is
        // emitted inline exactly as before.
        if self.capture_global_reads.is_some() {
            self.pending_disabled_commands.push((qualified, diag));
        } else {
            self.result.diagnostics.push(diag);
        }
    }

    /// Resolve a command's signature, honouring the active scoped command
    /// environment.
    ///
    /// Inside a scoped body (a `report::defstyle` style script, …) a scoped
    /// head (`top`, `columns`) resolves to its scoped signature; every other
    /// head — including the ordinary core commands used in the body
    /// (`set`, `split`, `string`) — falls back to the global registry.  This
    /// is the single scope-aware chokepoint that keeps the arity / subcommand
    /// emitters generic: they never learn a scoped command name.
    #[must_use]
    pub(in crate::analyser) fn resolve_command_signature(
        &self,
        cmd_name: &str,
        dialect: tcl_registry::prelude::DialectSet,
    ) -> Option<super::dispatch::CommandSignature> {
        if let Some(env) = self.body_scope_stack.last()
            && let Some(scoped) = env.command(cmd_name)
        {
            return Some(super::dispatch::signature_for_scoped_command(scoped));
        }
        let registry = self.registry.as_ref()?;
        super::dispatch::signature_for_command(registry, cmd_name, dialect)
    }

    pub(in crate::analyser) fn emit_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use super::dispatch::{CommandSignature, signature_for_command};
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(first_arg) = args.first() else {
            // Empty arg list — E001 path; not in scope here.
            return;
        };
        // Dynamic-value subcommand position — can't resolve statically.
        if arg_tokens
            .first()
            .is_some_and(|tok| has_substitution(first_arg, tok))
        {
            return;
        }
        // Tk geometry/widget ensemble commands (`grid` / `pack` / `wm` / …)
        // are recognised for the unknown-subcommand check regardless of the
        // active Tcl dialect — a `.tcl` script may `package require Tk` at
        // runtime, and W001 fires on `grid bogus` under every dialect.
        let dialect =
            DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL) | DialectSet::TK;
        // Scope-aware resolution: an ensemble scoped command (`top`, `data`)
        // inside a `report::defstyle` body is checked against its scoped
        // subcommand set, so `top bogus` still draws W001.
        let Some(CommandSignature::WithSubcommands(sig)) =
            self.resolve_command_signature(cmd_name, dialect)
        else {
            return;
        };
        if sig.allow_unknown {
            return;
        }
        // `after` dispatches on `cancel` / `idle` / `info`, but its first word
        // may instead be a millisecond delay (`after 200 {…}`).  An integer
        // first word is a valid time argument, not an unknown subcommand, so
        // it must not trip W001.  (Non-integer, non-subcommand words such as
        // `after foo` remain genuine errors and still fire.)
        if cmd_name == "after" && is_integer_word(first_arg) {
            return;
        }
        // A subcommand name never starts with `.`, so a `.`-prefixed first word
        // is a Tk window pathname, not an unknown subcommand.  This covers both
        // the geometry-manager shortcut (`grid .w ?args?` for `grid configure
        // .w …`, per grid.n / pack.n / place.n) and widget-creation commands
        // (`entry .e …`, `canvas .c …`), whose registry `subcommands` describe
        // the created widget's *instance* command rather than a first-word
        // subcommand of the creator.  Either way `.path` is never W001.
        if first_arg.starts_with('.') {
            return;
        }
        // Accept a unique-prefix abbreviation (`string le` ⇒ `length`), the way
        // Tcl's ensemble dispatch does, so valid abbreviations don't trip W001.
        if sig.is_known(first_arg) {
            return;
        }
        // The subcommand is unknown *in the active dialect*.  Before reporting
        // it as nonexistent, check whether it exists in some *other* dialect —
        // e.g. `info cmdtype` is a real subcommand introduced in Tcl 9.0 but
        // absent from the default 8.6 profile (issue #812).  That is the
        // subcommand-level analogue of the W002 disabled-in-dialect check for
        // whole commands (`emit_w002_disabled_command`): it EXISTS, just not
        // here, so it must be reported as disabled-in-dialect rather than as an
        // "Unknown subcommand" with a misleading spelling suggestion.
        if let Some(CommandSignature::WithSubcommands(any_sig)) =
            signature_for_command(registry, cmd_name, DialectSet::all())
            && any_sig.is_known(first_arg)
        {
            let span = match arg_tokens.first() {
                Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
                None => cmd_tok.span,
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W002,
                span,
                message: format!(
                    "'{cmd_name} {first_arg}' is disabled in the active dialect profile"
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
            return;
        }
        let mut message = format!("Unknown subcommand '{first_arg}' for '{cmd_name}'");
        let candidates: Vec<&str> = sig.subcommands.keys().map(String::as_str).collect();
        let suggestions = crate::text::suggest_similar(first_arg, candidates.iter().copied(), 1, 3);
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some(sub_tok) = arg_tokens.first() {
                // Target the *content* range of the subcommand
                // token rather than its full span.  Wrapper tokens
                // (`Str` braced, `Esc` quoted) carry the opening
                // delimiter via ``content_offset`` and intentionally
                // exclude the closing delimiter from ``span.end``;
                // replacing the full span would leave a stray
                // ``}`` / ``"`` behind (e.g. ``string {lenght}`` →
                // ``string length}``).  Using the content range
                // ([span.start + content_offset, span.end)) gives
                // ``{length}`` / ``"length"`` for the wrapped forms
                // and remains identical to the full span for bare
                // ``Esc`` words (``content_offset == 0``).
                let content_start = sub_tok.span.start() + u32::from(sub_tok.content_offset);
                let fix_span = tcl_lexer::Span::new(content_start, sub_tok.span.end());
                fixes.push(super::types::CodeFix {
                    span: fix_span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
        }
        // Anchor at the command-head + subcommand-name range so
        // the squiggle covers ``cmd subname`` rather than the
        // entire invocation: combine the command token with the
        // subcommand arg token.
        let span = match arg_tokens.first() {
            Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
            None => cmd_tok.span,
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W001,
            span,
            message,
            severity: Severity::Warning,
            fixes,
        });
    }

    /// **E002 / E003.** Argument-count check for simple (non-
    /// subcommand) commands: skip leading declared
    /// option flags, then compare the positional-argument count
    /// against the registry signature's arity bounds.
    ///
    /// Option skipping uses the dialect-filtered
    /// [`CommandSig::leading_options`](super::dispatch::CommandSig::leading_options)
    /// set, so switches introduced in a later Tcl release (e.g.
    /// `regsub -command`, 9.0+) are only skipped under a dialect that
    /// declares them.  This prevents both a false positive (declared
    /// switches counted as positional → spurious E003) and a dialect
    /// leak (9.0-only switches skipped under 8.x).
    ///
    /// `arg_expand[i]` marks an argument preceded by the Tcl 8.5+
    /// `{*}` expansion prefix.  A `{*}`-expanded word contributes an
    /// unknown number of runtime arguments, so option skipping stops
    /// at the first such word and the positional upper bound becomes
    /// unbounded — only the count of *non-expanded* positional words
    /// can still trip E003.
    ///
    /// **Intentional gaps:**
    /// - The `leading_options` skip is name-only, so the *value*
    ///   of a value-taking leading option is **not** skipped.
    /// - Statically-resolvable literal `{*}` expansions (`{*}{a b c}`)
    ///   are not refined to their element count; the conservative form
    ///   here can miss a genuine over-arity but never invents a false
    ///   positive.
    ///
    /// Subcommand-dispatch commands are handled by
    /// [`Self::emit_w001_unknown_subcommand`] and skipped here;
    /// per-subcommand arity is not checked.
    pub(in crate::analyser) fn emit_arity_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand_in: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        use super::dispatch::CommandSignature;
        use tcl_registry::prelude::DialectSet;

        // `arg_expand_in` is parallel to the full argv (command name at
        // index 0); drop that slot so it lines up with `args`.
        let arg_expand: &[bool] = arg_expand_in.get(1..).unwrap_or(&[]);

        // Same-file proc / TclOO forward / `interp alias` / static
        // `rename` arity — queued unconditionally, independent of the
        // registry resolution below, since a user proc can shadow a
        // builtin name (`proc ::ns::close {...}` inside `::ns`).
        self.queue_user_call_arity_candidate(
            cmd_name,
            &ArityWords {
                args,
                arg_tokens,
                arg_expand,
                cmd_tok,
            },
            scope_path,
        );

        // `TclOO` constructor-call arity (`ClassName new ?args?` /
        // `ClassName create name ?args?` / `ClassName createWithNamespace
        // name ::ns ?args?`) — queued unconditionally whenever the first
        // word is literally one of those three keywords, independent of
        // whether `cmd_name` resolves to anything at all; a call whose head
        // isn't a locally-known class is silently dropped at flush time.
        if matches!(
            args.first().map(String::as_str),
            Some("new" | "create" | "createWithNamespace")
        ) {
            self.queue_ctor_arity_candidate(
                cmd_name,
                &ArityWords {
                    args,
                    arg_tokens,
                    arg_expand,
                    cmd_tok,
                },
                scope_path,
            );
        }

        // `apply {{params} body} ?arg ...?` — a direct call to an inline
        // lambda. Unlike the two candidates above, the lambda's own arity
        // is fully knowable at the call site (no forward reference, no
        // cross-item merge to wait for), so this checks synchronously
        // rather than through the pending/flush queue.
        self.emit_apply_lambda_arity(cmd_name, args, arg_tokens, arg_expand, cmd_tok, scope_path);

        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        // Scope-aware: a head inside a scoped command environment resolves to
        // its scoped signature (`top set …`, `columns`), everything else to the
        // global registry.
        match self.resolve_command_signature(cmd_name, dialect) {
            Some(CommandSignature::Simple(sig)) => {
                self.check_simple_arity(
                    cmd_name,
                    cmd_name,
                    &sig,
                    &ArityWords {
                        args,
                        arg_tokens,
                        arg_expand,
                        cmd_tok,
                    },
                    scope_path,
                );
            }
            Some(CommandSignature::WithSubcommands(sig)) => {
                // Per-subcommand arity on `args[1:]`.  The W001
                // unknown-subcommand path is handled separately by
                // [`Self::emit_w001_unknown_subcommand`].
                let Some(sub_name) = args.first() else {
                    // **E001.** A subcommand-dispatch command invoked with no
                    // subcommand at all (`string` / `dict` / `info` on its
                    // own). Skipped when the registry's `subcommand_required`
                    // is `false` — a bare call has a well-defined default
                    // (e.g. `history` == `history info`), so it is not an
                    // arity error at all, not merely a suppressed one.
                    // Otherwise queued as a `pending_arity` candidate so an
                    // earlier shadowing user proc / class / alias / ensemble
                    // / stub suppresses it, exactly like the E002 / E003
                    // paths.
                    if !sig.subcommand_required {
                        return;
                    }
                    let ns = self.command_resolution_namespace(scope_path);
                    let enforce_order = !self.scope_path_in_proc_body(scope_path);
                    self.pending_arity.push((
                        cmd_name.to_string(),
                        ns,
                        enforce_order,
                        super::types::Diagnostic {
                            code: DiagCode::E001,
                            span: cmd_tok.span,
                            message: format!("'{cmd_name}' requires a subcommand"),
                            severity: Severity::Error,
                            fixes: Vec::new(),
                        },
                    ));
                    return;
                };
                // A `{*}`-expanded subcommand word resolves to an unknown
                // name at runtime; skip resolution and arity entirely.
                if arg_expand.first().copied().unwrap_or(false) {
                    return;
                }
                // Dynamic subcommand value — can't resolve statically.
                if arg_tokens
                    .first()
                    .is_some_and(|tok| has_substitution(sub_name, tok))
                {
                    return;
                }
                // Resolve exact-or-unique-prefix so an abbreviated subcommand
                // (`string le $s`) is arity-checked against `length`.
                let Some(sub_sig) = sig.resolve(sub_name) else {
                    // Unknown / ambiguous subcommand — W001's job, not arity.
                    return;
                };
                let display_name = format!("{cmd_name} {sub_name}");
                self.check_simple_arity(
                    cmd_name,
                    &display_name,
                    sub_sig,
                    &ArityWords {
                        args: &args[1..],
                        arg_tokens: arg_tokens.get(1..).unwrap_or(&[]),
                        arg_expand: arg_expand.get(1..).unwrap_or(&[]),
                        cmd_tok,
                    },
                    scope_path,
                );
            }
            None => {}
        }
    }

    /// Compare a positional-argument count against a single
    /// [`CommandSig`]'s arity bounds and queue an E002 / E003
    /// candidate.  Shared by the simple-command and per-subcommand
    /// arity paths in [`Self::emit_arity_diagnostics`].
    ///
    /// `resolution_name` is the base command name used by the
    /// post-walk [`Self::flush_arity_diagnostics`] to honour a
    /// shadowing user proc / class / alias (e.g. `file` for the
    /// `file link` subcommand check), while `display_name` is the
    /// human-facing name shown in the message (`file link`).
    ///
    /// `args` / `arg_tokens` / `arg_expand` are the slices *after*
    /// whatever prefix the caller has already consumed (the command
    /// name for the simple path; the command name and subcommand word
    /// for the subcommand path), so the leading-option scan and
    /// positional count operate on the same coordinate system as
    /// `sig`.
    ///
    /// A `sig` carrying [`tcl_registry::Traits::STRUCTURALLY_CHECKED_ARITY`]
    /// (`if`) is skipped entirely: its registry `arity` is a descriptive
    /// floor only, and its dedicated structural diagnostic (E004) already
    /// covers every too-few-/malformed-shape case this generic check
    /// would otherwise duplicate.
    fn check_simple_arity(
        &mut self,
        resolution_name: &str,
        display_name: &str,
        sig: &super::dispatch::CommandSig,
        words: &ArityWords<'_>,
        scope_path: &[usize],
    ) {
        if sig
            .traits
            .contains(tcl_registry::Traits::STRUCTURALLY_CHECKED_ARITY)
        {
            return;
        }
        let ArityWords {
            args,
            arg_tokens,
            arg_expand,
            cmd_tok,
        } = *words;
        let expanded = |i: usize| arg_expand.get(i).copied().unwrap_or(false);

        // Skip leading declared option flags *and the value word(s) each
        // consumes*.  Stop at the first non-option word, the option
        // terminator `--` (consumed), or a `{*}`-expanded word (whose value
        // can't be classified).  Skipping the value words is what keeps a
        // value-taking option (`regsub -start 0 …`, `file link -symbolic
        // dst src`) from having its value counted as a positional argument
        // — the same `value_word_count` skip the W004 dialect-option loop
        // uses.
        let mut i = 0usize;
        while i < args.len() {
            if expanded(i) {
                break;
            }
            let arg = &args[i];
            if arg == "--" {
                i += 1;
                break;
            }
            if let Some(opt) = sig.leading_option_specs.iter().find(|o| o.matches(arg)) {
                // Skip the flag itself plus however many value words it
                // consumes at this position (0 for a bare flag).
                i += 1 + opt.value_word_count(args, i);
            } else if sig.leading_options.contains(arg) {
                // Recognised as an option name but no spec carries its value
                // arity (e.g. a form-only or generated option) — skip just
                // the flag word, matching the prior name-only behaviour.
                i += 1;
            } else {
                break;
            }
        }
        let positional_start = i.min(args.len());
        let (nargs_min, positional_any_expand) =
            count_positionals(args, arg_expand, positional_start);

        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };

        // Capture the call-site command-resolution namespace so the
        // post-walk flush can resolve this command the Tcl way (current
        // namespace → global) and only suppress the arity check when
        // the call actually resolves to a user definition — not to any
        // same-tail-named proc elsewhere in the file. Uses the proc's
        // *defining* namespace (so `close` inside a body of
        // `proc ::ns::x` resolves through `::ns`), not just lexical
        // `namespace eval` nesting.
        let ns = self.command_resolution_namespace(scope_path);

        // Top-level calls (module body, `namespace eval` bodies, and
        // conditionals) execute in source order during load, so a
        // shadowing proc only silences the builtin arity check when its
        // definition lexically precedes the call.  Calls inside a proc
        // body resolve after the whole script has loaded, so order is
        // not enforced there.
        let enforce_order = !self.scope_path_in_proc_body(scope_path);

        // Collect as a *candidate*; the post-walk
        // [`Self::flush_arity_diagnostics`] drops it if the call
        // resolves to a user proc / class / alias / ensemble / stub.
        // A class / alias / ensemble / stub match suppresses regardless
        // of definition order; a *proc* match additionally honours
        // `enforce_order` (in-order/reachability gate).
        if let Some(diag) = arity_verdict(
            display_name,
            sig.arity,
            nargs_min,
            positional_any_expand,
            full_span,
        ) {
            self.pending_arity
                .push((resolution_name.to_string(), ns, enforce_order, diag));
        }
    }

    /// **E002 / E003** for `apply {{params} body} ?arg ...?`: check the
    /// trailing arguments against the inline lambda's *own* declared
    /// parameter list — the same "wrong # args" `TclOO`/`proc` argument
    /// binding rules apply to a lambda (confirmed against tclsh 9.0.4:
    /// `apply {{a b} {}} 1` fails `wrong # args: should be "apply
    /// lambdaExpr a b"`).
    ///
    /// A no-op when the first argument isn't a *braced* literal lambda
    /// (matching [`Analyser::parse_apply_lambda_elements`]'s guard — a
    /// dynamic `apply $lambda …` is opaque and left unchecked, the same
    /// any-uncertainty-abstains convention as every other arity path here).
    ///
    /// Queued through [`Self::pending_arity`] rather than pushed
    /// immediately: `apply` is an ordinary command name and can be shadowed
    /// by a user `proc apply {lambda x} {…}` exactly like any other
    /// builtin (confirmed against tclsh 9.0.4 — a user-defined `apply`
    /// resolves ahead of the language builtin), so this candidate must go
    /// through the same post-walk builtin-shadowing suppression
    /// (`Self::flush_arity_diagnostics`) as every other simple-command
    /// arity check, rather than bypassing it.
    fn emit_apply_lambda_arity(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let Some(elements) = self.parse_apply_lambda_elements(cmd_name, args, arg_tokens) else {
            return;
        };
        let Some((_, params_text)) = elements.first() else {
            return;
        };
        let params = crate::signature_scan::params::parse_param_list(params_text);
        let arity = crate::signature_scan::arity::arity_of(&params);
        // Positional count starts *after* the lambda literal (index 1).
        let (nargs_min, positional_any_expand) = count_positionals(args, arg_expand, 1);
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        if let Some(diag) =
            arity_verdict("apply", arity, nargs_min, positional_any_expand, full_span)
        {
            let ns = self.command_resolution_namespace(scope_path);
            let enforce_order = !self.scope_path_in_proc_body(scope_path);
            self.pending_arity
                .push((cmd_name.to_string(), ns, enforce_order, diag));
        }
    }

    /// Post-walk flush of the [`Self::pending_arity`] candidates
    /// collected by [`Self::emit_arity_diagnostics`].
    ///
    /// Runs after the command walk completes, when `all_procs`,
    /// `all_classes`, `command_aliases`, `ensemble_namespaces` and the
    /// inline stub set are fully populated.  A candidate is dropped
    /// only when the call **resolves to** a user definition rather than
    /// the builtin whose registry arity produced it — resolution
    /// follows Tcl's rule for unqualified commands (the call-site
    /// namespace, then global `::`), using the namespace captured at
    /// emit time.  So `proc ::ns::close {...}` suppresses a `close`
    /// call inside `::ns` (and a qualified `::ns::close ...`), but a
    /// `close` call in another namespace still resolves to the builtin
    /// and is checked.  Document-global declarations — inline
    /// `# tcl-lsp: stub`s — suppress by bare name regardless of
    /// namespace.
    ///
    /// Suppression by a shadowing **proc** also honours definition
    /// reachability: a top-level call (one whose
    /// `enforce_order` flag is set — module body, `namespace eval`
    /// body, or a conditional) is silenced only when the proc's
    /// definition lexically precedes it, since top-level commands run
    /// in source order during load (so a `close x y z` *before* a later
    /// `proc close` still reaches the builtin).  Proc-body calls run
    /// after load and are not order-gated.  Classes / aliases /
    /// ensembles / stubs always exist at run time and are never
    /// order-gated.  (Excluding *conditionally* defined procs would
    /// need the CFG dominator model, which is not modelled here.)
    ///
    /// Emit the per-item path's pending W002 (disabled-in-dialect command)
    /// diagnostics, re-applying the user-proc-shadowing suppression against the
    /// merged `all_procs` (a cross-item fact unavailable to an isolated body).
    /// No-op on the whole-file `analyse` path (W002 is emitted inline there, so
    /// `pending_disabled_commands` is empty) — keeping the two paths
    /// byte-identical.  The position guard (`name_span.start() < call.start()`)
    /// matches the inline check, so a unique-named proc resolves identically
    /// whether checked inline or here (duplicate proc names already force the
    /// per-item path to fall back).
    pub(in crate::analyser) fn flush_disabled_command_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_disabled_commands);
        for (qualified, diag) in pending {
            if let Some(def) = self.result.all_procs.get(&qualified)
                && def.name_span.start() < diag.span.start()
            {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// Idempotent: drains `pending_arity` and `pending_user_call_arity`,
    /// so a second call is a no-op.
    pub fn flush_arity_diagnostics(&mut self) {
        if self.pending_arity.is_empty() && self.pending_user_call_arity.is_empty() {
            return;
        }
        // Fully-qualified non-proc user-command names the calls may
        // resolve to (classes / aliases keyed by qualified name;
        // ensemble namespaces *are* the command name).  These always
        // exist by the time the script runs, so they suppress the
        // builtin arity check regardless of definition order.
        let mut non_proc_qnames: FxHashSet<&str> = FxHashSet::default();
        non_proc_qnames.extend(self.result.all_classes.keys().map(String::as_str));
        non_proc_qnames.extend(self.result.command_aliases.keys().map(String::as_str));
        non_proc_qnames.extend(self.ensemble_namespaces.iter().map(String::as_str));
        // Qualified proc name → definition offset (the proc-name
        // token start).  A shadowing proc only silences a *top-level*
        // call (`enforce_order`) when its definition lexically
        // precedes the call; proc-body calls are not order-gated.
        // Conditional / nested definitions are still treated as
        // shadowing here — distinguishing unconditionally-reachable
        // definitions needs the CFG dominator model, which is not
        // modelled here.
        let proc_offsets: FxHashMap<&str, u32> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, def)| (qname.as_str(), def.name_span.start()))
            .collect();
        // Inline stubs are document-global and unqualified.
        let stub_names = super::utils::scan_stub_command_names(&self.source);

        let pending = std::mem::take(&mut self.pending_arity);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let bare = cmd_name.rsplit("::").next().unwrap_or(&cmd_name);
            // Candidate qualified names this call could resolve to.
            let candidates = qualify_candidates(&ns, &cmd_name);
            // A proc shadows only when reachable at the call: top-level
            // calls require the definition to lexically precede them
            // (`def_off < call_off`); proc-body calls accept any
            // same-named definition.  Classes / aliases / ensembles /
            // stubs are not order-gated.
            let call_off = diag.span.start();
            let resolves_to_user = candidates.iter().any(|c| {
                non_proc_qnames.contains(c.as_str())
                    || proc_offsets
                        .get(c.as_str())
                        .is_some_and(|&def_off| !enforce_order || def_off < call_off)
            }) || stub_names.contains(bare);
            if resolves_to_user {
                continue;
            }
            self.result.diagnostics.push(diag);
        }

        // Same-file proc / TclOO forward / `interp alias` / static
        // `rename` arity — resolved now that `all_procs`,
        // `command_aliases`, and `renamed_commands` are fully populated
        // (post cross-item merge, same as the drain above). Unlike the
        // builtin path, there is nothing to *suppress* here: a candidate
        // either resolves to a definite arity (and is checked) or it
        // doesn't (and is silently dropped — a class / ensemble / stub /
        // genuinely unknown name, or a dynamic rename/alias target,
        // exactly like the registry path's own abstention rules).
        let user_pending = std::mem::take(&mut self.pending_user_call_arity);
        for cand in user_pending {
            let bare = cand.cmd_name.rsplit("::").next().unwrap_or(&cand.cmd_name);
            if stub_names.contains(bare) {
                continue;
            }
            let Some(arity) = self.resolve_indirect_call_target(&cand, &proc_offsets) else {
                continue;
            };
            if let Some(diag) = arity_verdict(
                &cand.cmd_name,
                arity,
                cand.nargs_min,
                cand.positional_any_expand,
                cand.full_span,
            ) {
                self.result.diagnostics.push(diag);
            }
        }
    }

    /// Idempotent: drains `pending_ctor_arity`, so a second call is a no-op.
    ///
    /// Resolves each queued `ClassName new` / `ClassName create name` /
    /// `ClassName createWithNamespace name ::ns` candidate against
    /// `all_classes` (fully populated post-walk, after every per-item body
    /// has been grafted) using the same current-namespace-then-global
    /// resolution and top-level order gate as the same-file proc/alias
    /// arity path. Only genuine `TclOO` classes (`oo::class` /
    /// `oo::configurable` / `oo::abstract` / `oo::singleton` metaclasses)
    /// are checked — snit/itcl classes use an entirely different
    /// instantiation protocol (`TypeName instanceName ?args?`, never
    /// `new`/`create`/`createWithNamespace`), so their (unrelated) calls,
    /// if any, must not be arity-checked against a `TclOO` constructor.
    ///
    /// A candidate resolving to a class with no explicit constructor
    /// anywhere in its MRO is dropped — `TclOO`'s inherited default
    /// constructor accepts any argument count (confirmed against tclsh
    /// 9.0.4). Each form's mandatory leading words
    /// ([`super::types::CtorForm::extra_leading_words`]) are folded into
    /// the expected bound (`bump_arity`) before comparison, so the arity
    /// message reads in terms of the call's own full argument list, not
    /// the constructor's.
    pub fn flush_ctor_arity_diagnostics(&mut self) {
        if self.pending_ctor_arity.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_ctor_arity);
        let mut diags: Vec<super::types::Diagnostic> = Vec::new();
        {
            // Scoped so the `class_hierarchy()` borrow of `self.result` ends
            // before the diagnostics are pushed onto `self.result.diagnostics`
            // below — sharing the memoised hierarchy (built once per analysis,
            // reused by hover / hierarchy LSP providers) rather than rebuilding
            // an owned copy just for this pass.
            let hierarchy = self.result.class_hierarchy();
            for cand in &pending {
                let candidates = qualify_candidates(&cand.ns, &cand.class_name);
                let Some((class_qn, cd)) = candidates.iter().find_map(|c| {
                    let cd = self.result.all_classes.get(c)?;
                    let in_effect = !cand.enforce_order || cd.name_span.start() < cand.call_off;
                    in_effect.then_some((c.as_str(), cd))
                }) else {
                    continue; // not a (yet-defined) class call — nothing to check
                };
                if !is_tcloo_metaclass(self.registry.as_ref(), &cd.metaclass) {
                    continue; // snit / itcl — `new`/`create` mean something else
                }
                // Unlike `new`/`create` (exported by default — only an explicit
                // `unexport` blocks an external call), `createWithNamespace` is
                // *unexported by default*: an external `ClassName
                // createWithNamespace …` (every call this candidate queue can
                // even see — a literal `my createWithNamespace` never resolves
                // `my` as a class name, so it never reaches here) raises
                // "unknown method" and never touches the constructor unless the
                // class explicitly `export`s it (confirmed against this
                // project's own `runtime/rust/src/cmd_oo.rs`'s
                // `oo_class_factory`: `cwn_ok = !block_unexported ||
                // cwn_exp`). Order-insensitively checking the whole file's
                // `exports` set (rather than only exports in effect by
                // `cand.call_off`) trades a vanishingly rare false negative — an
                // export that lexically follows a top-level call — for
                // eliminating a real false positive on every default-unexported
                // class, consistent with this pass's abstain-when-unsure
                // convention.
                if matches!(cand.form, super::types::CtorForm::CreateWithNamespace)
                    && !cd.exports.contains("createWithNamespace")
                {
                    continue;
                }
                let Some(provider) = hierarchy.constructor_provider(class_qn, &self.source) else {
                    continue; // no explicit constructor anywhere in the MRO —
                    // TclOO's inherited default accepts any argument count
                };
                // `constructor_provider` picked `provider` from its *final*
                // (last-declared) constructor only — re-select within it,
                // honouring both the empty-body exclusion and (for a
                // top-level call) definition order: a class created via
                // `oo::class create Foo {}` and only later given a
                // `constructor` through a separate `oo::define Foo { … }`
                // has no constructor in effect for any call between the
                // two (confirmed against tclsh 9.0.4). A redefinition
                // mid-file is honoured the same way — the constructor
                // *in effect at the call site*, not simply the last one
                // written anywhere in the file, the same convention
                // `resolve_indirect_call_target` uses for a same-file proc.
                // When the immediate provider has no qualifying entry, this
                // abstains rather than walking further up the MRO for an
                // ancestor's constructor that might have been in effect —
                // a conservative, sound-by-abstention simplification for
                // this rare a combination (order-sensitive call *and*
                // multiple inheritance/redefinition), not a soundness gap.
                let Some(ctor) = self.result.all_classes.get(provider).and_then(|cd| {
                    cd.constructors.iter().rev().find(|c| {
                        !super::class_hierarchy::is_empty_method_body(&self.source, c.body_span)
                            && (!cand.enforce_order || c.name_span.start() < cand.call_off)
                    })
                }) else {
                    continue;
                };
                let arity = crate::signature_scan::arity::arity_of(&ctor.params);
                let arity = bump_arity(arity, cand.form.extra_leading_words());
                let display_name = format!("{} {}", cand.class_name, cand.form.as_str());
                if let Some(diag) = arity_verdict(
                    &display_name,
                    arity,
                    cand.nargs_min,
                    cand.positional_any_expand,
                    cand.full_span,
                ) {
                    diags.push(diag);
                }
            }
        }
        self.result.diagnostics.extend(diags);
    }

    /// Queue a same-file user-call arity candidate for every command
    /// invocation, independent of whether it also resolves to a
    /// registry signature — [`Self::flush_arity_diagnostics`] resolves
    /// it post-walk against same-file procs / `TclOO` forwards / `interp
    /// alias` / static `rename` targets it can't see yet mid-walk
    /// (forward references, cross-item merging). A call that turns out
    /// to resolve to nothing with a known arity (a builtin, a class, an
    /// ensemble, a stub, or simply unresolved) is silently dropped at
    /// flush time — this queue never invents a diagnostic the resolver
    /// can't back up.
    ///
    /// User procs have no declared option flags, so — unlike
    /// [`Self::check_simple_arity`] — there is no leading-option skip:
    /// every word is positional from index 0.
    fn queue_user_call_arity_candidate(
        &mut self,
        cmd_name: &str,
        words: &ArityWords<'_>,
        scope_path: &[usize],
    ) {
        if cmd_name.is_empty() || cmd_name.contains(['$', '[']) {
            return; // dynamic command name — nothing to resolve statically
        }
        let ArityWords {
            args,
            arg_tokens,
            arg_expand,
            cmd_tok,
        } = *words;
        let (nargs_min, positional_any_expand) = count_positionals(args, arg_expand, 0);
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        self.pending_user_call_arity.push(PendingUserCallArity {
            cmd_name: cmd_name.to_string(),
            ns: self.command_resolution_namespace(scope_path),
            enforce_order: !self.scope_path_in_proc_body(scope_path),
            call_off: cmd_tok.span.start(),
            full_span,
            nargs_min,
            positional_any_expand,
        });
    }

    /// Queue a `TclOO` constructor-call (`ClassName new ?args?` /
    /// `ClassName create name ?args?`) arity candidate. Queued
    /// unconditionally by every call whose first word is
    /// `new`/`create`/`createWithNamespace` (the caller's guard) —
    /// [`Self::flush_ctor_arity_diagnostics`] resolves it post-walk against
    /// `all_classes`, which mid-walk may not yet hold a forward-referenced
    /// class. A call whose head doesn't resolve to a locally-known class is
    /// silently dropped at flush time, exactly like
    /// [`Self::queue_user_call_arity_candidate`].
    ///
    /// `words.args` still has the keyword at index 0; the positional count
    /// starts at index 1 so the keyword itself is never counted.
    fn queue_ctor_arity_candidate(
        &mut self,
        cmd_name: &str,
        words: &ArityWords<'_>,
        scope_path: &[usize],
    ) {
        if cmd_name.is_empty() || cmd_name.contains(['$', '[']) {
            return; // dynamic class name — nothing to resolve statically
        }
        let ArityWords {
            args,
            arg_tokens,
            arg_expand,
            cmd_tok,
        } = *words;
        let form = match args[0].as_str() {
            "create" => super::types::CtorForm::Create,
            "createWithNamespace" => super::types::CtorForm::CreateWithNamespace,
            _ => super::types::CtorForm::New,
        };
        let (nargs_min, positional_any_expand) = count_positionals(args, arg_expand, 1);
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        self.pending_ctor_arity
            .push(super::types::PendingCtorArity {
                class_name: cmd_name.to_string(),
                ns: self.command_resolution_namespace(scope_path),
                enforce_order: !self.scope_path_in_proc_body(scope_path),
                form,
                call_off: cmd_tok.span.start(),
                full_span,
                nargs_min,
                positional_any_expand,
            });
    }

    /// Whether a fact (a proc definition, `rename`, or `interp alias`)
    /// established at `established_off` is observably in effect by the
    /// time `cand`'s call executes: unconditionally true inside a
    /// proc/method body (the whole file loads, running every top-level
    /// statement, before any body runs), and order-gated by textual
    /// offset at top level (confirmed against tclsh 9.0.4: a top-level
    /// call textually before the statement that establishes a proc /
    /// rename / alias executes first at run time, so the fact isn't in
    /// effect there yet).
    fn fact_in_effect(cand: &PendingUserCallArity, established_off: u32) -> bool {
        !cand.enforce_order || established_off < cand.call_off
    }

    /// Chase `cand.cmd_name` (as it resolves at `cand.ns`) through
    /// same-file proc / static `rename` / `interp alias` indirection to
    /// a definite [`Arity`], or `None` when nothing with a known arity
    /// is reached.
    ///
    /// Each hop is namespace-qualified via [`qualify_candidates`] — the
    /// same resolution order as the builtin-shadowing suppression check
    /// in [`Self::flush_arity_diagnostics`]. A proc target, a `rename`
    /// target, and an `interp alias` target are all order-gated via
    /// [`Self::fact_in_effect`] (`proc_offsets` for procs,
    /// `rename_offsets` / `alias_offsets` for the other two) — a
    /// candidate whose defining statement hasn't executed yet at a
    /// top-level call site is not a match. A name `rename`d away
    /// (`deleted_commands`) is skipped as a proc match once the rename
    /// is in effect — it no longer denotes that proc at run time (see
    /// [`crate::analyser::handlers::Analyser::handle_rename`]).
    /// `interp alias`'s prepended arguments shift the eventual arity
    /// down (real partial application, confirmed against tclsh 9.0.4);
    /// chained aliases/renames accumulate the shift transitively.
    /// Hop-limited as a defensive guard against a self-referential
    /// rename/alias cycle (never legitimate Tcl). Reaching a registry
    /// builtin only counts once at least one hop has happened — a
    /// *direct* hit on a builtin name is [`Self::check_simple_arity`]'s
    /// job, not this one's.
    fn resolve_indirect_call_target(
        &self,
        cand: &PendingUserCallArity,
        proc_offsets: &FxHashMap<&str, u32>,
    ) -> Option<Arity> {
        const MAX_HOPS: u8 = 8;
        let mut cur = cand.cmd_name.clone();
        let mut prepended_total: u16 = 0;
        let mut hopped = false;
        // Whether `cur` was just reached via a *rename* hop (as opposed
        // to being the original call name or an alias hop's target).
        // `rename OLD NEW` moves the command's identity to `NEW` once
        // and for all, so chasing `NEW` back to `OLD` to read its
        // original `all_procs` entry is always valid regardless of
        // `OLD`'s own deletion — `OLD` being deleted is precisely what
        // freed it up to serve as this rename's source. An *alias*
        // target, by contrast, is re-resolved by name every time it's
        // invoked (confirmed against tclsh 9.0.4: `interp alias {} bar
        // {} foo` then `rename foo baz` — or `rename foo {}` — makes
        // `bar` fail too, "invalid command name foo"), so the deletion
        // check applies there exactly as it does to the original call
        // name.
        let mut via_rename_hop = false;
        for _ in 0..MAX_HOPS {
            let candidates = qualify_candidates(&cand.ns, &cur);
            for c in &candidates {
                if !via_rename_hop
                    && self
                        .deleted_commands
                        .get(c.as_str())
                        .is_some_and(|&off| Self::fact_in_effect(cand, off))
                {
                    continue;
                }
                if let Some(def) = self.result.all_procs.get(c)
                    && (!cand.enforce_order
                        || proc_offsets
                            .get(c.as_str())
                            .is_some_and(|&off| off < cand.call_off))
                {
                    let arity = crate::signature_scan::arity::arity_of(&def.params);
                    return Some(shift_arity(arity, prepended_total));
                }
            }
            if let Some(old) = candidates.iter().find_map(|c| {
                let old = self.renamed_commands.get(c)?;
                let off = *self.rename_offsets.get(c)?;
                Self::fact_in_effect(cand, off).then_some(old)
            }) {
                cur.clone_from(old);
                hopped = true;
                via_rename_hop = true;
                continue;
            }
            if let Some((target, prepended)) = candidates.iter().find_map(|c| {
                // `interp alias srcPath srcCmd {}` (or a `rename c {}`)
                // deletes an alias exactly like it deletes a proc — same
                // blanket "any in-effect deletion wins" convention as the
                // proc branch above (`via_rename_hop` doesn't apply here;
                // an alias is re-resolved by name on every call, never
                // chased back through a rename the way a proc's original
                // definition is).
                if self
                    .deleted_commands
                    .get(c.as_str())
                    .is_some_and(|&off| Self::fact_in_effect(cand, off))
                {
                    return None;
                }
                let alias = self.command_aliases.get(c)?;
                let off = *self.alias_offsets.get(c)?;
                Self::fact_in_effect(cand, off).then_some(alias)
            }) {
                prepended_total = prepended_total
                    .saturating_add(u16::try_from(prepended.len()).unwrap_or(u16::MAX));
                cur.clone_from(target);
                hopped = true;
                via_rename_hop = false;
                continue;
            }
            if hopped {
                // Mirrors `command_binding.rs::default_binding`: only an
                // unqualified global name the registry knows is a
                // builtin.
                let bare = cur.strip_prefix("::").unwrap_or(&cur);
                if !bare.contains("::")
                    && let Some(sig) = self.registry.as_ref().and_then(|r| r.get(bare))
                {
                    return Some(shift_arity(sig.arity, prepended_total));
                }
            }
            return None;
        }
        None
    }

    /// **E004.** Emit a precise "malformed `if`" diagnostic from a
    /// registry [`tcl_registry::ClauseShapeChecker`] hook — the grammar
    /// walk itself lives once, in `tcl-registry`
    /// (`commands::tcl::if_::walk_if`), shared with the `if_arg_roles`
    /// highlighting resolver, so this emitter never re-parses `if`'s
    /// shape independently.
    ///
    /// Dispatched generically off the resolved command spec's
    /// `clause_shape_check` hook (see
    /// [`Self::emit_dispatch_site_diagnostics`]) rather than
    /// `cmd_name == "if"`, so a namespace-qualified `::if` is covered
    /// too — registry name resolution already normalises the leading
    /// `::` for every command. `if` is the only hook today, so the
    /// diagnostic code below is hardcoded to `E004`; a second hook
    /// consumer would need this to carry its own code.
    ///
    /// Verified against Tcl 9.0.4's `TclNRIfObjCmd` /
    /// `IfConditionCallback` (`generic/tclCmdIL.c`) and tclsh 8.6 (same
    /// algorithm): a leading `else`/`elseif` bareword condition
    /// (`if else {a}`) is *not* a malformed `if` — it is a well-formed
    /// `if` whose condition fails at expression-evaluation time (an
    /// invalid-bareword error), a distinct problem this diagnostic does
    /// not own.
    ///
    /// Anchors on the offending word(s) — the dangling keyword or
    /// condition for a missing expression/script, or just the extra
    /// words for a trailing-words error — not the whole `if` statement,
    /// which can span many lines in an `elseif` chain. Offers a code fix
    /// only where one is unambiguous:
    /// - **extra words** — merge them into the final recognised body by
    ///   wrapping the untouched source slice from that body through the
    ///   last extra word in one more brace pair (preserves original
    ///   word delimiters and whitespace verbatim; no re-parsing);
    /// - **a dangling `elseif`/`else` clause following at least one
    ///   complete clause** — remove it, restoring the last well-formed
    ///   prefix;
    /// - a missing *first* clause (`if` alone, or `if {cond}` with
    ///   nothing else) offers no fix: there is no well-formed prefix to
    ///   fall back to, and inventing a body would be a guess, not a
    ///   mechanical fix.
    pub(in crate::analyser) fn emit_e004_clause_shape_diagnostic(
        &mut self,
        cmd_name: &str,
        checker: tcl_registry::ClauseShapeChecker,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::ClauseShapeError;

        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(error) = checker(&arg_strs) else {
            return;
        };

        let word_span = |i: usize| {
            arg_tokens
                .get(i)
                .map_or(cmd_tok.span, |t| widened_word_span(*t, &self.source))
        };

        let (span, message) = match error {
            ClauseShapeError::MissingExpr { after: None } => (
                cmd_tok.span,
                format!("No expression after \"{cmd_name}\" argument"),
            ),
            ClauseShapeError::MissingExpr { after: Some(i) } => (
                word_span(i),
                format!(
                    "No expression after \"{}\" argument",
                    args.get(i).map_or("", String::as_str)
                ),
            ),
            ClauseShapeError::MissingBody { after: i } => (
                word_span(i),
                format!(
                    "No script following \"{}\" argument",
                    args.get(i).map_or("", String::as_str)
                ),
            ),
            ClauseShapeError::ExtraWords { first_extra } => {
                let start = arg_tokens.get(first_extra).map(|t| t.span.start());
                let end = arg_tokens.last().map(|t| widen_token_end(*t, &self.source));
                let span = match (start, end) {
                    (Some(s), Some(e)) => tcl_lexer::Span::new(s, e),
                    _ => cmd_tok.span,
                };
                // Matches Tcl's own message text exactly: `Tcl_IfObjCmd`
                // builds this from a *static* string, always naming "if"
                // literally — never the invoked spelling (unlike the
                // other two messages above, which do use it).
                (
                    span,
                    "Extra words after \"else\" clause in \"if\" command".to_string(),
                )
            }
        };

        let fixes = self.e004_fixes(args, arg_tokens, error);

        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::E004,
            span,
            message,
            severity: Severity::Error,
            fixes,
        });
    }

    /// Code fixes for [`Self::emit_e004_clause_shape_diagnostic`]. See
    /// that method's doc comment for which cases get a fix and why.
    fn e004_fixes(
        &self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        error: tcl_registry::ClauseShapeError,
    ) -> Vec<super::types::CodeFix> {
        use tcl_registry::ClauseShapeError;

        match error {
            ClauseShapeError::ExtraWords { first_extra } => {
                // `first_extra >= 2` always: the walk only reaches
                // `ExtraWords` after consuming the mandatory first
                // clause's body (index >= 1), so the recognised final
                // body this merges from is always in range.
                let Some(body_tok) = arg_tokens.get(first_extra - 1) else {
                    return Vec::new();
                };
                let Some(last_tok) = arg_tokens.last() else {
                    return Vec::new();
                };
                let start = body_tok.span.start();
                let end = widen_token_end(*last_tok, &self.source);
                let Some(slice) = self.source.get(start as usize..end as usize) else {
                    return Vec::new();
                };
                vec![super::types::CodeFix {
                    span: tcl_lexer::Span::new(start, end),
                    new_text: format!("{{{slice}}}"),
                    description: "Merge trailing words into the if body".to_string(),
                }]
            }
            ClauseShapeError::MissingExpr {
                after: Some(kw_idx),
            } => {
                // A dangling `elseif` always follows a complete clause
                // (the walk only reaches the `elseif`/`else` lookup after
                // consuming one), so removing from the keyword onward
                // always restores a well-formed prefix.
                self.remove_dangling_clause_fix(kw_idx, arg_tokens)
            }
            ClauseShapeError::MissingBody { after } => {
                // `after` names the last present word of the *dangling*
                // clause (a condition, `then`, or `else`) — walk back to
                // the `elseif`/`else` keyword that opened it. Word 0 is
                // never a real keyword (it is always the mandatory first
                // condition, whatever its text), so the search excludes
                // it — `after` staying at 0 or 1 there means the very
                // first clause never completed, and no prefix fix
                // exists.
                let Some(start_idx) = (1..=after)
                    .rev()
                    .find(|&k| args.get(k).is_some_and(|w| w == "elseif" || w == "else"))
                else {
                    return Vec::new();
                };
                self.remove_dangling_clause_fix(start_idx, arg_tokens)
            }
            ClauseShapeError::MissingExpr { after: None } => Vec::new(),
        }
    }

    /// A [`super::types::CodeFix`] removing every word from
    /// `arg_tokens[start_idx]` through the end of the command —
    /// restoring the well-formed `if` prefix that precedes a dangling
    /// trailing clause.
    fn remove_dangling_clause_fix(
        &self,
        start_idx: usize,
        arg_tokens: &[tcl_lexer::Token],
    ) -> Vec<super::types::CodeFix> {
        let (Some(start_tok), Some(last_tok)) = (arg_tokens.get(start_idx), arg_tokens.last())
        else {
            return Vec::new();
        };
        let start = start_tok.span.start();
        let end = widen_token_end(*last_tok, &self.source);
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(start, end),
            new_text: String::new(),
            description: "Remove incomplete trailing clause".to_string(),
        }]
    }

    /// **W304.** Emit "Missing option terminator (`--`)" diagnostics
    /// for option-bearing commands whose first positional argument
    /// could be misinterpreted as an option.
    ///
    /// Resolves the command's option-
    /// terminator profile via
    /// [`tcl_registry::CommandRegistry::resolve_option_terminator`],
    /// scans for the first positional argument that lacks a
    /// preceding `--`, and emits a tristate-severity diagnostic:
    ///
    /// - **OFF** (no diagnostic) — the value is provably non-`-`-
    ///   prefixed (a non-dynamic literal whose representative token
    ///   isn't a `Var`/`Cmd` and whose text doesn't start with `-`).
    /// - **INFO** — dynamic value (`Var` / `Cmd` token) with no
    ///   proof of starting with `-`.  When the value is a single-
    ///   token `Var` whose most recent literal `set` resolves to a
    ///   non-`-`-prefixed value, an additional "origin" diagnostic
    ///   is emitted at the resolution site to explain the INFO
    ///   downgrade.
    /// - **WARNING** — the value is known to start with `-`: either
    ///   a literal whose first character is `-`, or a `Var` whose
    ///   constant-propagated value starts with `-`.
    ///
    /// The diagnostic carries a code-fix that prepends `"-- "` to
    /// the positional-argument span (with a one-byte extension for
    /// `Cmd` tokens whose lexer span excludes the closing `]`).
    ///
    /// **Note on `warn_without_terminator`:** the registry's
    /// `Traits::WARN_WITHOUT_TERMINATOR` flag (set on `regexp` only
    /// today) is plumbed onto [`tcl_registry::ResolvedTerminator`]
    /// but is not consumed.  The OFF gate
    /// fires uniformly for non-dynamic, non-`-`-prefixed values
    /// regardless of the trait.
    pub(in crate::analyser) fn emit_w304_missing_option_terminator(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }

        // Resolve the option-terminator profile *dialect-agnostically*:
        // resolving with no dialect means W304 still fires on a command
        // that the active dialect disables (e.g. `exec` / `glob` under
        // f5-irules, which also draw W002 / W123).  Passing the dialect
        // here would over-filter via `get_for_dialect` and silently drop
        // those W304s.
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(profile) =
            registry.resolve_option_terminator(cmd_name, &arg_strs, DialectSet::empty())
        else {
            return;
        };

        // The braced pattern-list switch form ``switch $x { pat body … }``
        // is NOT a runtime hazard: Tcl unambiguously identifies the
        // trailing brace as the pattern list and never consumes the
        // preceding word as an option.  Detect the two-arg braced form
        // (the last arg is a brace-enclosed `Str` token) and exempt it
        // entirely.  The SPLIT form (`switch $x -nocase {body} …`, 3+
        // args) is still flagged.
        if cmd_name == "switch"
            && arg_tokens.len() == 2
            && arg_tokens.last().map(|t| t.kind) == Some(tcl_lexer::TokenType::Str)
        {
            return;
        }

        let Some(positional_idx) = first_positional_without_terminator(args, &profile) else {
            return;
        };
        if positional_idx >= arg_tokens.len() {
            return;
        }

        let tok = arg_tokens[positional_idx];
        let text = &args[positional_idx];

        let is_dynamic = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        let looks_like_option = text.starts_with('-');

        // OFF — non-dynamic value that does not start with `-` can
        // never be confused with an option.
        if !is_dynamic && !looks_like_option {
            return;
        }

        let command_label = match profile.subcommand {
            Some(sub) => format!("{cmd_name} {sub}"),
            None => cmd_name.to_string(),
        };

        // Build the code-fix span.  For ``Cmd`` (`[…]`) tokens the
        // lexer span covers ``[inner`` but excludes the closing
        // ``]``; extend by one byte when the byte after ``span.end``
        // is ``]`` so the replacement encompasses the bracket pair.
        // (Body-local: the fix text is the argument's own source slice, so it is
        // computable in an isolated body and rebased by the graft.)
        let (fix_span, diag_end) = self.compute_w304_fix_span(tok);
        let fix_text = format!(
            "-- {}",
            &self.source[fix_span.start() as usize..fix_span.end() as usize]
        );
        let fixes = vec![super::types::CodeFix {
            span: fix_span,
            new_text: fix_text,
            description: "Insert '--' option terminator".to_string(),
        }];
        let diag_span = tcl_lexer::Span::new(tok.span.start(), diag_end);
        // Suppress unused-warning on the rare path where `cmd_tok`
        // isn't needed (the diagnostic anchors at the positional
        // arg's span, not the command head).
        let _ = cmd_tok;

        // The `Var` dynamic-not-option branch of `classify_w304` resolves the
        // variable against the most recent literal `set` in the *whole file*
        // (`last_literal_set_value_for_var` scans `self.source`).  An isolated
        // proc body's `self.source` is only the body, so an enclosing-scope set
        // would be missed.  On the per-item path, defer that one source-dependent
        // case to the tail (where `self.source` is the full file); every other
        // branch is body-local and emitted inline.
        if self.capture_global_reads.is_some()
            && is_dynamic
            && !looks_like_option
            && matches!(tok.kind, tcl_lexer::TokenType::Var)
        {
            self.pending_w304
                .push((tok, command_label, fixes, diag_span));
            return;
        }

        let (severity, message, origin) =
            self.classify_w304(tok, is_dynamic, looks_like_option, &command_label);
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W304,
            span: diag_span,
            message,
            severity,
            fixes,
        });
        if let Some(origin_diag) = origin {
            self.result.diagnostics.push(origin_diag);
        }
    }

    /// **W217.** An `unset` whose leading option words (`-nocomplain` / `--`)
    /// consume *every* argument, so the call unsets no variable at all.
    ///
    /// This is almost always a mistake: either a variable name was forgotten,
    /// or the author meant to unset a variable whose name begins with `-` (e.g.
    /// a variable literally named `-nocomplain`) and needs a `--` terminator in
    /// front of it — `unset -nocomplain` is parsed as the flag, not the name.
    ///
    /// Fires only when at least one option word was consumed **and** no variable
    /// name follows (`unset -nocomplain`, `unset --`, `unset -nocomplain --`).
    /// A call with any real variable (`unset -nocomplain $x`, `unset x`,
    /// `unset foo -nocomplain` where `-nocomplain` is a name) stays silent.
    /// Carries a fix that inserts `--` before the first option word so the
    /// remaining words are unset as variable names.
    pub(in crate::analyser) fn emit_w217_unset_option_only(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "unset" || args.is_empty() || arg_tokens.len() != args.len() {
            return;
        }
        // `unset` recognises only `-nocomplain` (skippable, repeatable) and `--`
        // (terminator); any other word ends option parsing and is a variable
        // name.  Mirrors `lower_unset` and the registry `unset` arg-role
        // resolver (verified against tclsh 8.6/9.0).
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-nocomplain" => i += 1,
                "--" => {
                    i += 1;
                    break;
                }
                _ => break,
            }
        }
        // Fire only when the options consumed every argument (no variable name
        // remains) after consuming at least one option word.
        if i == 0 || i < args.len() {
            return;
        }

        let first = arg_tokens[0];
        let last = arg_tokens[args.len() - 1];
        let diag_span = tcl_lexer::Span::new(first.span.start(), last.span.end());
        // Fix: prepend `--` to the first option word so every following word —
        // including a `-`-named variable — is unset as a variable name.
        let slice = &self.source[first.span.start() as usize..first.span.end() as usize];
        let fixes = vec![super::types::CodeFix {
            span: first.span,
            new_text: format!("-- {slice}"),
            description: "Insert '--' so the following words are variable names".to_string(),
        }];
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W217,
            span: diag_span,
            message: "`unset` unsets no variable here — `-nocomplain` / `--` are consumed as \
options. To unset a variable whose name begins with `-`, put `--` before it \
(e.g. `unset -- -nocomplain`)."
                .to_string(),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Emit the per-item path's pending W304 diagnostics, classifying each
    /// `$var` against the **full-file** most-recent-literal-`set` resolution
    /// (impossible inside an isolated body, whose `self.source` is only the
    /// body).  All inputs are absolute by the time the tail runs (the graft
    /// rebased the token, fix, and diagnostic spans), so the result is identical
    /// to the inline whole-file emission.  No-op on the `analyse` path
    /// (`pending_w304` empty).
    pub(in crate::analyser) fn flush_w304_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_w304);
        for (tok, command_label, fixes, diag_span) in pending {
            let (severity, message, origin) = self.classify_w304(tok, true, false, &command_label);
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W304,
                span: diag_span,
                message,
                severity,
                fixes,
            });
            if let Some(origin_diag) = origin {
                self.result.diagnostics.push(origin_diag);
            }
        }
    }

    /// **W116 / W117.** Stub command / expression definition shadows a
    /// built-in.  Post-walk check.  W116 fires when a `# tcl-lsp:
    /// stub` command name (with leading `::` stripped) collides with a
    /// registered command; W117 when a stub expr function/operator name
    /// collides with a built-in `expr` function or operator.
    pub(in crate::analyser) fn emit_w116_w117_stub_shadows(&mut self) {
        use super::types::{Diagnostic, Severity};

        if self.result.stub_commands.is_empty() && self.result.stub_expr_defs.is_empty() {
            return;
        }

        // W116 — stub command shadows a built-in command.  Build the
        // dialect command-name set locally.
        if !self.result.stub_commands.is_empty() {
            use tcl_registry::CommandRegistry;
            use tcl_registry::prelude::DialectSet;
            let mut registry = CommandRegistry::build_default();
            if let Some(d) = DialectSet::parse(&self.dialect) {
                registry.load_dialect(d);
            }
            let commands: std::collections::HashSet<&str> = registry.command_names().collect();
            let hits: Vec<(String, tcl_lexer::Span)> = self
                .result
                .stub_commands
                .iter()
                .filter(|s| commands.contains(s.name.trim_start_matches(':')))
                .map(|s| (s.name.clone(), s.range))
                .collect();
            for (name, span) in hits {
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::W116,
                    span,
                    message: format!("Stub command '{name}' shadows built-in command."),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }

        // W117 — stub expr function/operator shadows a built-in.
        if !self.result.stub_expr_defs.is_empty() {
            let irules = self.dialect == "f5-irules";
            let hits: Vec<(String, String, tcl_lexer::Span)> = self
                .result
                .stub_expr_defs
                .iter()
                .filter(|s| {
                    BUILTIN_MATH_FUNCTIONS.contains(&s.name.as_str())
                        || BUILTIN_EXPR_OPS.contains(&s.name.as_str())
                        || (irules && IRULES_EXPR_OPS.contains(&s.name.as_str()))
                })
                .map(|s| (s.name.clone(), s.kind.clone(), s.range))
                .collect();
            for (name, kind, span) in hits {
                let kind_label = if kind == "function" {
                    "function"
                } else {
                    "operator"
                };
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::W117,
                    span,
                    message: format!(
                        "Stub expression {kind_label} '{name}' shadows built-in {kind_label}."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **IRULE2002.** Warn when a deprecated iRules command is used —
    /// the command's spec carries a `deprecated_replacement`.  Only fires
    /// under the `f5-irules` dialect.
    pub(in crate::analyser) fn emit_irule2002_deprecated_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect != "f5-irules" {
            return;
        }
        let Some(replacement) = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| s.deprecated_replacement)
        else {
            return;
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::Irule2002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is deprecated in iRules. Use '{replacement}' instead."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE2001.** Warn that `matchclass` is deprecated — use
    /// `class match` instead.  Only fires under the `f5-irules` dialect.
    /// This fires *alongside* IRULE2002 at the same span (the
    /// command head): `matchclass` carries both a `deprecated_replacement`
    /// (→ IRULE2002) and a dedicated rule (→ IRULE2001).
    pub(in crate::analyser) fn emit_irule2001_matchclass(
        &mut self,
        cmd_name: &str,
        arg_tokens: &[tcl_lexer::Token],
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect != "f5-irules" || cmd_name != "matchclass" {
            return;
        }
        // Auto-fix `matchclass` → `class match`, a 1:1 rename (same argument
        // order).  The iRules forms are:
        //   * 3-arg `matchclass <item> <operator> <class>` → preserve all three
        //     verbatim as `class match <item> <operator> <class>`.
        //   * 2-arg shorthand `matchclass <item> <class>` → expand with the
        //     default operator: `class match <item> equals <class>`.
        // Any other arity is ambiguous, so we still warn but offer NO quick-fix
        // rather than corrupt the command.  (Gating on `>= 2` and always forcing
        // `equals` mangled the 3-arg form — e.g. `matchclass [HTTP::uri]
        // starts_with $::admin_paths` became `class match [HTTP::uri] equals
        // starts_with`, dropping the real class and operator.)  The raw source
        // slices preserve `$var` / `[cmd]` substitutions verbatim (the
        // substituted `args` values would drop them).  The lexer reports
        // representative spans for `[cmd …]` / `${name}` / `"…"` words without
        // their closing delimiter, so each slice — and the whole-command fix
        // range — is widened through trailing closers; otherwise
        // `[HTTP::uri]` would round-trip as `[HTTP::uri`.
        let word_end = |t: &tcl_lexer::Token| {
            crate::optimiser::helpers::spans::full_rewrite_span(&self.source, t.span).end()
        };
        let raw = |t: &tcl_lexer::Token| {
            self.source[t.span.start() as usize..word_end(t) as usize].to_string()
        };
        let new_text = match arg_tokens {
            [item, cls] => Some(format!("class match {} equals {}", raw(item), raw(cls))),
            [item, operator, cls] => Some(format!(
                "class match {} {} {}",
                raw(item),
                raw(operator),
                raw(cls)
            )),
            _ => None,
        };
        let fixes = new_text
            .map(|new_text| {
                let end = arg_tokens.last().map_or(cmd_tok.span.end(), word_end);
                vec![super::types::CodeFix {
                    span: tcl_lexer::Span::new(cmd_tok.span.start(), end),
                    new_text,
                    description: "Replace with 'class match'".to_string(),
                }]
            })
            .unwrap_or_default();
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::Irule2001,
            span: cmd_tok.span,
            message: "'matchclass' is deprecated since BIG-IP v10. \
Use 'class match <item> <operator> <class>' instead."
                .to_string(),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Classify the positional value for W304: tristate severity,
    /// human-readable message, and an optional "origin" diagnostic
    /// for the constant-propagated INFO path.  Split out of
    /// [`Self::emit_w304_missing_option_terminator`] to keep that
    /// method's body within the clippy `too_many_lines` budget.
    fn classify_w304(
        &self,
        tok: tcl_lexer::Token,
        is_dynamic: bool,
        looks_like_option: bool,
        command_label: &str,
    ) -> (Severity, String, Option<super::types::Diagnostic>) {
        if is_dynamic && !looks_like_option {
            if matches!(tok.kind, tcl_lexer::TokenType::Var) {
                let var_name = self.var_name_from_token(tok);
                let resolved = var_name.and_then(|name| {
                    last_literal_set_value_for_var(
                        &self.source,
                        &name,
                        tok.span.start(),
                        self.lexer_config(),
                    )
                });
                if let Some((resolved_text, resolved_span, var_text)) = resolved {
                    if resolved_text.starts_with('-') {
                        let message = format!(
                            "'{command_label}' parses leading '-' as options. \
This value currently resolves to '{resolved_text}', so add '--' to force \
data parsing."
                        );
                        return (Severity::Warning, message, None);
                    }
                    let message = format!(
                        "'{command_label}' parses leading '-' as options. \
This value is reported at INFO because '{var_text}' currently resolves to \
static literal '{resolved_text}'. Keep '--' to guard against future \
option-injection regressions if the variable changes."
                    );
                    let origin = super::types::Diagnostic {
                        code: DiagCode::W304,
                        span: resolved_span,
                        message: format!(
                            "'{var_text}' is currently assigned static \
literal '{resolved_text}' here; this is why the diagnostic is INFO."
                        ),
                        severity: Severity::Suggestion,
                        fixes: Vec::new(),
                    };
                    return (Severity::Suggestion, message, Some(origin));
                }
            }
            // Command substitution / unresolved variable — INFO
            // with the substituted-input message.
            let message = format!(
                "'{command_label}' parses leading '-' as options. \
Insert '--' before substituted input to reduce option-injection risk."
            );
            return (Severity::Suggestion, message, None);
        }
        // ALWAYS: literal value that starts with `-`.
        let message = format!(
            "'{command_label}' argument starts with '-'. Add '--' \
before this value so it is treated as data, not an option."
        );
        (Severity::Warning, message, None)
    }

    /// Extract the variable name for a `Var` token using the
    /// lexer-provided token-text semantics
    /// ([`tcl_lexer::SourceMap::token_text`]).  Preserves the
    /// `Var`-specific normalisation rules (notably the trailing
    /// `}` strip for the `${}` degenerate case where the lexer
    /// extends the span by one byte to cover the closing brace),
    /// so this stays in sync with the rest of the analyser's
    /// token-text usage and avoids edge-case mismatches that a
    /// raw `self.source[..]` slice would introduce.  Returns
    /// `None` when the extracted text is empty.
    fn var_name_from_token(&self, tok: tcl_lexer::Token) -> Option<String> {
        let sm = tcl_lexer::SourceMap::new(&self.source);
        let text = sm.token_text(tok);
        if text.is_empty() {
            return None;
        }
        Some(text.to_string())
    }

    /// Compute the W304 code-fix span and diagnostic end position.
    ///
    /// For `Cmd` tokens (`[…]`) the lexer span excludes the closing
    /// `]`; we extend the span by one byte when the next character
    /// is `]` so the prepended ``-- `` doesn't split the bracket
    /// pair.  All other token kinds use the lexer span directly.
    fn compute_w304_fix_span(&self, tok: tcl_lexer::Token) -> (tcl_lexer::Span, u32) {
        let span_start = tok.span.start();
        let span_end = tok.span.end();
        if matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            let after = span_end as usize;
            if after < self.source.len() && self.source.as_bytes()[after] == b']' {
                let extended = span_end + 1;
                return (tcl_lexer::Span::new(span_start, extended), extended);
            }
        }
        (tcl_lexer::Span::new(span_start, span_end), span_end)
    }

    /// **W004.** Emit "Command option is not available in the active
    /// dialect" warning for option-bearing commands invoked with an
    /// option whose registry entry restricts it to a dialect that
    /// doesn't include the active one.
    ///
    /// Examples:
    /// `lsearch -stride` on Tcl 8.4 / 8.5 (option is 8.6+),
    /// `regsub -command` / `clock scan -validate` /
    /// `fconfigure -nodelay` on Tcl 8.x (options are 9.0+).
    ///
    /// Walks args looking for `-foo`-shaped flags, asks the registry
    /// for the matching `OptionSpec`, and fires when
    /// `OptionSpec::supports_dialect` returns false.  Substituted
    /// flag values (`-foo $bar`, `-foo [cmd]`) are skipped because
    /// the dispatching is only on the *flag name*; we don't have to
    /// inspect the value.  `--` terminates the scan.
    ///
    /// Subcommand-scoped options consult the subcommand's
    /// `OptionSpec` table when the first arg matches a known
    /// subcommand.
    pub(in crate::analyser) fn emit_w004_dialect_invalid_option(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::dialects::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };

        // Resolve subcommand-level options when the first arg names
        // one.
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| spec.subcommands.iter().find(|s| s.name == args[0].as_str()))
            .flatten();
        let (options, parent_dialects, start_idx) = if let Some(sub) = sub_match {
            (sub.options, sub.dialects.or(spec.dialects), 1usize)
        } else {
            (spec.options, spec.dialects, 0usize)
        };

        if options.is_empty() {
            return;
        }

        let mut i = start_idx;
        while i < args.len() {
            let arg = args[i].as_str();
            if arg == "--" {
                break;
            }
            if !arg.starts_with('-') || arg.len() < 2 {
                i += 1;
                continue;
            }
            // Skip negative number literals (`-1`, `-1.5`).
            let rest = &arg[1..].trim_start_matches('-');
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                i += 1;
                continue;
            }
            // Skip dynamic-value args (Var / Cmd tokens).  The flag
            // name itself comes from the arg text, but if the
            // representative token is a substitution we can't know
            // it's actually `-foo`.
            if i < arg_tokens.len() {
                let tok = arg_tokens[i];
                if matches!(
                    tok.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                ) {
                    i += 1;
                    continue;
                }
            }
            // Find a matching OptionSpec (canonical name or alias).  When it is
            // dialect-gated out, emit W004.  Either way, skip the value word(s)
            // it consumes, so a value that itself looks like a flag
            // (`-command -bar`) is not mistakenly tested as an option.
            if let Some(opt) = options.iter().find(|o| o.matches(arg)) {
                if !opt.supports_dialect(Some(active), parent_dialects) && i < arg_tokens.len() {
                    let span = arg_tokens[i].span;
                    // Message exactly: `Option 'X' on 'cmd'[ sub] is not
                    // available in the active dialect (D).`
                    let sub_suffix = sub_match.map_or(String::new(), |s| format!(" {}", s.name));
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: DiagCode::W004,
                        span,
                        message: format!(
                            "Option '{arg}' on '{cmd_name}'{sub_suffix} is not available \
in the active dialect ({}).",
                            self.dialect
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
                i += 1 + opt.value_word_count(args, i);
                continue;
            }
            i += 1;
        }
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warning for expressions that use a Tcl 9.0 string-
    /// comparison operator (`lt` / `le` / `gt` / `ge`, TIP 461) in a
    /// pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl 8.5+) in
    /// Tcl 8.4 / f5-irules.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        use tcl_registry::dialects::DialectSet;

        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        // Pre-Tcl-8.5 dialects don't accept `in` / `ni` (TIP 201).
        let pre_85 = !DialectSet::TCL85_PLUS.contains(active);
        // Pre-Tcl-9.0 dialects don't accept `lt` / `le` / `gt` / `ge`
        // (TIP 461); 9.0 and 9.1 both do.
        let pre_90 = !DialectSet::TCL90_PLUS.contains(active);
        if !pre_85 && !pre_90 {
            return;
        }

        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let mut found: Vec<&'static str> = Vec::new();
        walk_dialect_invalid_ops(&parsed, pre_85, pre_90, &mut found);
        for op_name in found {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W003,
                span: diag_span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'.",
                    self.dialect
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }
}

/// Built-in `expr` math functions.  Used by the
/// W117 stub-shadow check.
const BUILTIN_MATH_FUNCTIONS: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "atan2",
    "bool",
    "ceil",
    "cos",
    "cosh",
    "double",
    "entier",
    "exp",
    "floor",
    "fmod",
    "hypot",
    "int",
    "isinf",
    "isnan",
    "isqrt",
    "log",
    "log10",
    "max",
    "min",
    "pow",
    "rand",
    "round",
    "sin",
    "sinh",
    "sqrt",
    "srand",
    "tan",
    "tanh",
    "wide",
    // Tcl 9.1 C99 math functions (TIP 745), verified against
    // `tmp/tcl9.1-src/changes.md`.  The multi-value C99 functions land as the
    // `divmod`/`frexp`/`modf`/`remquo` *commands* instead.
    "acosh",
    "asinh",
    "atanh",
    "cbrt",
    "copysign",
    "dim",
    "erf",
    "erfc",
    "exp2",
    "expm1",
    "fma",
    "gamma",
    "ldexp",
    "lgamma",
    "log1p",
    "log2",
    "logb",
    "nextafter",
    "remainder",
    "signbit",
    "trunc",
];

/// Built-in `expr` operators.
const BUILTIN_EXPR_OPS: &[&str] = &[
    "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=", ">>",
    "^", "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "||", "~",
];

/// iRules-only `expr` operators.
const IRULES_EXPR_OPS: &[&str] = &[
    "and",
    "contains",
    "ends_with",
    "equals",
    "matches_glob",
    "matches_regex",
    "not",
    "or",
    "starts_with",
];

/// Scan `args` for the first positional argument that lacks a
/// preceding `--` terminator.
///
/// Skips option words (text starts with `-`); skips an additional
/// argument when the option's [`OptionSpec`](tcl_registry::prelude::OptionSpec)
/// in [`ResolvedTerminator::options`](tcl_registry::ResolvedTerminator)
/// has `takes_value == true`.  Linear scan over the borrowed
/// option slice — per-command option counts are small (≤ a dozen
/// for the largest specs in practice), so this is cheaper than a
/// per-resolve `HashSet` allocation on the analyser hot path.
/// Returns `None` when a `--` is encountered (positional arguments
/// after `--` are explicitly terminated).
fn first_positional_without_terminator(
    args: &[String],
    profile: &tcl_registry::ResolvedTerminator,
) -> Option<usize> {
    let mut i = profile.scan_start;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            // Skip the option and the value word(s) it consumes (arity-aware).
            let consumed = profile
                .options
                .iter()
                .find(|o| o.matches(arg))
                .map_or(0, |o| o.value_word_count(args, i));
            i += 1 + consumed;
            continue;
        }
        return Some(i);
    }
    None
}

/// Locate the most-recent literal `set var value` assignment whose
/// command-head precedes `before_offset`.
///
/// Returns `Some((value_text, value_span, var_text))` when the
/// nearest preceding `set` is a fully-literal three-arg form.
/// Returns `None` when the latest assignment is dynamic / multi-
/// token (the runtime value cannot be proven statically).
fn last_literal_set_value_for_var(
    source: &str,
    var_name: &str,
    before_offset: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, tcl_lexer::Span, String)> {
    if var_name.is_empty() || before_offset == 0 {
        return None;
    }
    let head = before_offset as usize;
    if head > source.len() {
        return None;
    }
    let prefix = &source[..head];
    let segments = crate::segmenter::segment_commands_with_offset_and_config(prefix, 0, config);

    for cmd in segments.iter().rev() {
        // Cross-scope guard: stop the backward scan at a `proc NAME
        // {PARAMS} BODY` whose body *contains* the use offset and whose
        // params include `var_name` — the parameter shadows any outer
        // scope, so an outer `set` must not be attributed to the inner
        // use.  The use is inside the proc body iff that proc is the one
        // left unclosed by the truncation at `before_offset`: its span
        // then reaches the last truncated byte (`end + 1 >= head`).  A
        // *complete* proc before the use ends well before that and does
        // not shadow.
        let use_inside_proc = cmd.span.end() as usize + 1 >= head;
        if use_inside_proc
            && cmd.texts.first().map(String::as_str) == Some("proc")
            && cmd.texts.len() >= 4
            && cmd.texts[2].contains(var_name)
        {
            let shadows = crate::tcl_expr_eval::split_tcl_list(&cmd.texts[2])
                .iter()
                .any(|el| el.split_whitespace().next() == Some(var_name));
            if shadows {
                return None;
            }
        }

        if cmd.texts.first().map(String::as_str) != Some("set") {
            continue;
        }
        if cmd.texts.len() < 3 {
            continue;
        }
        if cmd.texts[1] != var_name {
            continue;
        }
        // Most recent assignment wins.  If it's dynamic, the
        // runtime value can't be proven statically.
        if cmd.single_token_word.get(2).copied() != Some(true) {
            return None;
        }
        if cmd.argv.len() < 3 {
            return None;
        }
        let value_tok = cmd.argv[2];
        if !matches!(
            value_tok.kind,
            tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
        ) {
            return None;
        }
        return Some((cmd.texts[2].clone(), value_tok.span, var_name.to_string()));
    }
    None
}

/// Return `true` if `text` contains any of the dialect-gated
/// expression operator keywords (`lt`, `le`, `gt`, `ge`, `in`, `ni`)
/// as a whole word — i.e. surrounded by non-identifier bytes or
/// the text boundary.  Used as a fast prefilter to skip the
/// expression parse for expressions that obviously can't trigger
/// W003.
///
/// Whitespace-aware: tabs, newlines, and any other non-identifier
/// byte (parentheses, operators, comparison glyphs, etc.) count
/// as word boundaries.  Matches Tcl expr's tolerance for
/// arbitrary whitespace between tokens.
pub(super) fn contains_gated_word(text: &str) -> bool {
    const GATED: &[&[u8]] = &[b"lt", b"le", b"gt", b"ge", b"in", b"ni"];
    let bytes = text.as_bytes();
    for needle in GATED {
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == *needle {
                let before_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
                let after_ok = i + n == bytes.len() || !is_ident_continue(bytes[i + n]);
                if before_ok && after_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

fn walk_dialect_invalid_ops(
    node: &ExprNode,
    pre_85: bool,
    pre_90: bool,
    found: &mut Vec<&'static str>,
) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_dialect_invalid_ops(left, pre_85, pre_90, found);
            walk_dialect_invalid_ops(right, pre_85, pre_90, found);
            match op {
                BinOp::In if pre_85 => found.push("in"),
                BinOp::Ni if pre_85 => found.push("ni"),
                BinOp::StrLt if pre_90 => found.push("lt"),
                BinOp::StrLe if pre_90 => found.push("le"),
                BinOp::StrGt if pre_90 => found.push("gt"),
                BinOp::StrGe if pre_90 => found.push("ge"),
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            walk_dialect_invalid_ops(operand, pre_85, pre_90, found);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_dialect_invalid_ops(condition, pre_85, pre_90, found);
            walk_dialect_invalid_ops(true_branch, pre_85, pre_90, found);
            walk_dialect_invalid_ops(false_branch, pre_85, pre_90, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_dialect_invalid_ops(arg, pre_85, pre_90, found);
            }
        }
        _ => {}
    }
}
