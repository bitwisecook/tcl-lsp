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
use tcl_registry::{Arity, ProfileQueries};

use super::helpers::{has_substitution, is_ident_continue};
use crate::analyser::state::Analyser;
use crate::analyser::types::{PendingUserCallArity, Severity};
use crate::expr_ast::{ExprNode, render_expr};

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
/// The tight E003 anchor: the span covering the run of *surplus*
/// positional arguments (from the first argument past the command's `max`
/// up to the last argument), plus where a "remove surplus arguments" fix
/// should start deleting — the end of the last *kept* word, so the fix
/// also removes the separating whitespace.
#[derive(Clone, Copy)]
pub(super) struct ExcessArgs {
    /// Span of the surplus argument run (diagnostic anchor).
    pub span: tcl_lexer::Span,
    /// Deletion start for the removal fix (end of the preceding word).
    pub delete_from: u32,
}

/// Compute the [`ExcessArgs`] anchor. `positional_start` is the index of
/// the first positional word (after any leading option flags), `max` the
/// command's maximum positional count, `fallback_delete_from` the widened
/// end of whatever word precedes the first positional (the command head or
/// subcommand word) for the `max == 0` case. Returns `None` — so
/// [`arity_verdict`] falls back to the whole-command span — when the
/// positional region contains a `{*}` expansion (which makes "the first
/// surplus word" ambiguous) or the index arithmetic can't land on a real
/// token.
fn excess_positional_span(
    arg_tokens: &[tcl_lexer::Token],
    arg_expand: &[bool],
    positional_start: usize,
    max: usize,
    source: &str,
    fallback_delete_from: u32,
) -> Option<ExcessArgs> {
    if (positional_start..arg_tokens.len()).any(|i| arg_expand.get(i).copied().unwrap_or(false)) {
        return None;
    }
    let first_excess = positional_start.checked_add(max)?;
    let first = arg_tokens.get(first_excess)?;
    let last = arg_tokens.last()?;
    let delete_from = match first_excess.checked_sub(1).and_then(|i| arg_tokens.get(i)) {
        Some(prev) => widen_token_end(*prev, source),
        None => fallback_delete_from,
    };
    Some(ExcessArgs {
        span: tcl_lexer::Span::new(first.span.start(), widen_token_end(*last, source)),
        delete_from,
    })
}

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
/// whatever encloses this word). Any other token kind
/// (a bareword, `then`/`elseif`/`else`, a `Var`, …) has no closer to
/// widen for.
fn widen_token_end(tok: tcl_lexer::Token, source: &str) -> u32 {
    let closer = match tok.kind {
        tcl_lexer::TokenType::Str => b'}',
        tcl_lexer::TokenType::Cmd => b']',
        _ => return tok.span.end(),
    };
    let end = tok.span.end();
    // Empty content (`{}` / `[]` / `""`) — the span's end already sits past
    // the closer.  Detected arithmetically (content start reaching the end)
    // rather than by slicing the source, so a token whose span outruns the
    // supplied text (unit tests drive the walk with detached tokens) never
    // panics.
    if tok.span.start() + u32::from(tok.content_offset) >= end {
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

/// Two shape-based reasons `first_arg` is never W001 regardless of whether
/// it names a real subcommand of the resolved ensemble — split out of
/// [`Analyser::emit_w001_unknown_subcommand`] to keep that function under
/// the line-count lint.
/// Which option table an option scan runs against, and how its dispatch
/// treats abbreviated spellings.
struct OptionScanContext {
    /// The resolved option table (command-level, or the subcommand's).
    options: &'static [tcl_registry::hover::OptionSpec],
    /// Dialect set the options inherit when they declare none.
    parent_dialects: Option<tcl_registry::prelude::DialectSet>,
    /// First argument index the scan starts at (1 past a subcommand word).
    start_idx: usize,
    /// The resolved subcommand name, for messages.
    sub_name: Option<&'static str>,
    /// Whether abbreviations resolve in this table.
    prefix_matching: tcl_registry::abbrev::PrefixMatching,
}

/// The dialect-available options (canonical spellings plus declared aliases)
/// as a [`tcl_registry::abbrev::KeywordTable`], for abbreviation resolution.
///
/// Tcl's own option table holds the aliases, so `-bd` prefix-matches exactly
/// like `-borderwidth`.
fn option_keyword_table(
    options: &'static [tcl_registry::hover::OptionSpec],
    available: impl Fn(&tcl_registry::hover::OptionSpec) -> bool,
    prefix_matching: tcl_registry::abbrev::PrefixMatching,
) -> tcl_registry::abbrev::KeywordTable<'static> {
    tcl_registry::abbrev::KeywordTable::from_keywords(
        options.iter().filter(|opt| available(opt)).flat_map(|opt| {
            std::iter::once(opt.name)
                .chain(opt.aliases.iter().copied())
                .map(move |name| tcl_registry::abbrev::Keyword {
                    name,
                    min_abbrev: opt.min_abbrev,
                })
        }),
        prefix_matching,
    )
}

fn shape_exempt_from_w001(sig: &super::dispatch::SubcommandSig, first_arg: &str) -> bool {
    // A command may declare a *default* form selected by a first word of a
    // particular value shape rather than a subcommand — `after` dispatches
    // on `cancel` / `idle` / `info`, but an integer first word is a
    // millisecond delay (`after 200 {…}`), not an unknown subcommand. The
    // shape comes from the spec's `default_form_first_word`; no command is
    // named here. (Non-matching words such as `after foo` remain genuine
    // errors and still fire.)
    if sig.matches_default_form(first_arg) {
        return true;
    }
    // A subcommand name never starts with `.`, so a `.`-prefixed first word
    // is a Tk window pathname, not an unknown subcommand.  This covers both
    // the geometry-manager shortcut (`grid .w ?args?` for `grid configure
    // .w …`, per grid.n / pack.n / place.n) and widget-creation commands
    // (`entry .e …`, `canvas .c …`), whose registry `subcommands` describe
    // the created widget's *instance* command rather than a first-word
    // subcommand of the creator.  Either way `.path` is never W001.
    first_arg.starts_with('.')
}

/// Describe [`Arity`]'s [`Arity::step`] / [`Arity::also_exact`] shape in
/// prose for the E005 message — `"0, 2, 4, …"` for a plain progression,
/// `"2, or 3, 5, 7, …"` when `also_exact` adds an exception (`switch`'s
/// shorthand-or-pairs union). Only called when `step != 0` (a caller with
/// no parity constraint never reaches the E005 branch).
fn describe_step_shape(arity: Arity) -> String {
    let min = u32::from(arity.min);
    let step = u32::from(arity.step);
    let progression = format!("{min}, {}, {}, …", min + step, min + step * 2);
    match arity.also_exact {
        Some(exact) => format!("{exact}, or {progression}"),
        None => progression,
    }
}

/// Compare a resolved [`Arity`] against an observed positional-argument
/// count and build the E002 / E003 / E005 diagnostic, or `None` when the
/// count fits. Shared by the registry-command arity path
/// ([`Analyser::check_simple_arity`]), the same-file proc / `TclOO`
/// method / `interp alias` / `rename` arity path
/// ([`Analyser::flush_arity_diagnostics`]), and the `TclOO` method-call
/// arity check ([`super::var_command`]), so all three diagnostics carry
/// identical wording.
///
/// **E005** fires when `nargs_min` is within `[min, max]` (so neither
/// E002 nor E003 applies) but doesn't land on [`Arity::step`]'s
/// progression or match [`Arity::also_exact`] — a count that's
/// individually "enough" but doesn't fit the command's key/value-pair /
/// paired-argument shape (`dict create a` — one word, not a key/value
/// pair; `foreach a b c d` — an unpaired trailing var-list). Same
/// `{*}`-expansion abstention as E002: an expanded tail's true final
/// count is unknowable, so a merely-in-range lower bound proves nothing
/// about its parity either.
///
/// `usage` is the resolved spec's primary synopsis; when present it is
/// appended to every verdict as a " — usage: …" suffix so the message
/// shows the expected call shape, not just the counts. Callers with no
/// registry spec (same-file procs, `TclOO` constructors/methods, `apply`
/// lambdas) pass `None` and keep the count-only message.
pub(super) fn arity_verdict(
    display_name: &str,
    arity: Arity,
    nargs_min: usize,
    positional_any_expand: bool,
    span: tcl_lexer::Span,
    excess: Option<ExcessArgs>,
    usage: Option<&str>,
) -> Option<crate::analyser::types::Diagnostic> {
    let usage_suffix = usage.map(|u| format!(" — usage: {u}")).unwrap_or_default();
    let min = usize::from(arity.min);
    let max = usize::from(arity.max);
    // `also_exact` is valid regardless of `min`/`max`/`step` — checked
    // first so it exempts the shorthand count from every branch below,
    // not just E005 (`switch $s {p1 b1 p2 b2}`'s 2-arg form must not
    // trip E002 just because `switch`'s own `min` is 3).
    if arity
        .also_exact
        .is_some_and(|exact| usize::from(exact) == nargs_min)
    {
        return None;
    }
    if !positional_any_expand && nargs_min < min {
        Some(crate::analyser::types::Diagnostic {
            code: DiagCode::E002,
            span,
            message: format!(
                "Too few arguments for '{display_name}': expected at least {min}, \
got {nargs_min}{usage_suffix}"
            ),
            severity: Severity::Error,
            fixes: Vec::new(),
        })
    } else if !arity.is_unlimited() && nargs_min > max {
        // Highlight only the surplus arguments when the caller could
        // isolate them (the whole command otherwise), and offer to delete
        // exactly that run (plus the separating whitespace). E002/E005 keep
        // the whole-command anchor: a too-few or wrong-shape count has no
        // specific surplus word to point at.
        let (e003_span, fixes) = match &excess {
            Some(ex) => (
                ex.span,
                vec![crate::analyser::types::CodeFix {
                    span: tcl_lexer::Span::new(ex.delete_from, ex.span.end()),
                    new_text: String::new(),
                    description: "Remove surplus argument(s)".to_string(),
                    // E003: deleting the surplus words assumes they were the mistake
                    // rather than a missing option that would have consumed them.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }],
            ),
            None => (span, Vec::new()),
        };
        Some(crate::analyser::types::Diagnostic {
            code: DiagCode::E003,
            span: e003_span,
            message: format!(
                "Too many arguments for '{display_name}': expected at most {max}, \
got {nargs_min}{usage_suffix}"
            ),
            severity: Severity::Error,
            fixes,
        })
    } else if !positional_any_expand
        && arity.step != 0
        && !(nargs_min - min).is_multiple_of(usize::from(arity.step))
    {
        Some(crate::analyser::types::Diagnostic {
            code: DiagCode::E005,
            span,
            message: format!(
                "Wrong argument-count shape for '{display_name}': expected {}, \
got {nargs_min}{usage_suffix}",
                describe_step_shape(arity)
            ),
            severity: Severity::Error,
            fixes: Vec::new(),
        })
    } else {
        None
    }
}

/// Format the "(available in: …)" suffix for a W002 message from a
/// command/subcommand's registry-declared dialect restriction — entirely
/// data-driven from [`tcl_registry::CommandSpec::dialects`] /
/// [`tcl_registry::SubCommand::dialects`], never a per-command name check.
/// Empty when `dialects` is `None` (unrestricted) or has no primitive
/// member (defensive; a restricted spec always has at least one).
fn dialect_availability_suffix(dialects: Option<tcl_registry::prelude::DialectSet>) -> String {
    let Some(dialects) = dialects else {
        return String::new();
    };
    let names = dialects.member_names();
    if names.is_empty() {
        return String::new();
    }
    format!(" (available in: {})", names.join(", "))
}

/// The *content* range of a subcommand word token — excluding a wrapper
/// delimiter (`{…}` / `"…"`) when present.  Wrapper tokens carry the opening
/// delimiter via `content_offset` and intentionally exclude the closing
/// delimiter from `span.end` (see the word-token closing-delimiter
/// convention); using the content range gives `length` (not `{length}` /
/// `"length"`) for a wrapped subcommand and is identical to the full span for
/// a bare `Esc` word (`content_offset == 0`). Shared by the W001 diagnostic
/// span and its "did you mean" [`CodeFix`](super::types::CodeFix) span so the
/// squiggle and the quick-fix target the same text.
fn subcommand_content_span(tok: tcl_lexer::Token) -> tcl_lexer::Span {
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    tcl_lexer::Span::new(content_start, tok.span.end())
}

/// Namespace-qualify `cmd_name`'s resolution candidates the Tcl way: current
/// namespace first, then global. Shared by the builtin-shadowing suppression
/// check ([`Analyser::flush_arity_diagnostics`]) and the same-file
/// proc/alias/rename resolution chase
/// ([`Analyser::resolve_indirect_call_target`]), so both walk the identical
/// candidate order — and, via [`crate::naming::bareword_resolution_candidates`],
/// the identical order the optimiser's interprocedural proc resolution uses,
/// so a fix to the rule can't drift between the two.
fn qualify_candidates(ns: &str, cmd_name: &str) -> Vec<String> {
    crate::naming::bareword_resolution_candidates(ns, cmd_name)
}

/// [`qualify_candidates`] plus the **implicit `namespace path`** in force at
/// `call_off` — today only a `TclOO` member body's `::oo::Helpers`, and that
/// name comes from the registry
/// ([`tcl_registry::definer::TCLOO_MEMBER_BODY_NAMESPACE_PATH`], read through
/// [`crate::analyser::scope::implicit_command_namespace_path_at`]), never from
/// a literal here.
///
/// Without the path a bare word written in a method body only ever qualifies
/// to `::WORD`, so a document that installs its own
/// `proc ::oo::Helpers::callback` — the documented "`TclOO` Tricks" wiki
/// helper `ticklecharts` uses under 8.6 — did not count as shadowing the
/// like-named 9.0 builtin, and the call drew a W002 "disabled in the active
/// dialect profile" the program provably never sees (issue #923 audit,
/// `ticklecharts` idx 51). tclsh 8.6.14 runs that file: the helper *is* the
/// command the method body reaches.
fn qualify_candidates_with_path(ns: &str, cmd_name: &str, path: &[&str]) -> Vec<String> {
    crate::naming::command_resolution_candidates(ns, path, cmd_name)
}

/// Whole-file facts needed to decide whether a same-file call resolves to a
/// user definition rather than falling through to a registry builtin.
///
/// Shared by the two same-file "does this call resolve away from the
/// builtin" suppression checks — [`Analyser::flush_arity_diagnostics`]
/// (suppress a builtin arity mismatch) and
/// [`Analyser::flush_disabled_command_diagnostics`] (suppress a
/// disabled-in-dialect warning) — both need the identical resolution rule,
/// so a fix to one (e.g. the `rename`-target gap this closed) automatically
/// applies to the other. Built once per flush pass over the fully-merged
/// post-walk facts (`all_procs` / `all_classes` / `command_aliases` /
/// `renamed_commands` / `ensemble_namespaces` — populated cross-item after
/// every per-item body has been grafted).
///
/// Owns its data (rather than borrowing `Analyser`) so a flush loop can
/// build it once up front and then freely take/mutate other `Analyser`
/// fields (`pending_arity`, `result.diagnostics`, …) while iterating —
/// borrowing through a whole-`&Analyser` build call would otherwise pin the
/// borrow checker's view to "all of `self`" for the facts' lifetime.
pub(super) struct UserResolutionFacts {
    /// Fully-qualified names that unconditionally denote a user command once
    /// declared anywhere in the file: `TclOO`/snit/itcl classes, `interp
    /// alias` names, and `namespace ensemble create` namespaces. Not
    /// order-gated — a deliberately permissive, false-negative-avoiding
    /// convention (a genuinely order-violating call is rare enough that
    /// abstaining from a real builtin-mismatch report is the safer trade).
    non_proc_qnames: FxHashSet<String>,
    /// Qualified proc name → the offset of its *earliest* declaration —
    /// the point from which the name denotes a user command at all. A name
    /// declared twice keeps the first offset here (`all_procs` alone only
    /// remembers the second), so a call between the two still suppresses the
    /// builtin-mismatch diagnostic it shadows (issue #923 idx 45).
    proc_offsets: FxHashMap<String, u32>,
    /// Qualified *new* name (a static `rename OLD NEW`) → the `rename`
    /// statement's token offset. `rename` moves an existing command's
    /// identity onto `NEW`, so once in effect `NEW` is a real command
    /// exactly like a proc definition — order-gated the same way.
    rename_offsets: FxHashMap<String, u32>,
    /// Document-global `# tcl-lsp: stub` command names (unqualified).
    stub_names: std::collections::HashSet<String>,
}

impl UserResolutionFacts {
    pub(super) fn build(a: &Analyser) -> Self {
        let mut non_proc_qnames: FxHashSet<String> = FxHashSet::default();
        non_proc_qnames.extend(a.result.all_classes.keys().cloned());
        non_proc_qnames.extend(a.result.command_aliases.keys().cloned());
        non_proc_qnames.extend(a.ensemble_namespaces.iter().cloned());
        let proc_offsets: FxHashMap<String, u32> = a
            .result
            .all_procs
            .keys()
            .map(|qname| {
                let earliest = a
                    .result
                    .proc_declarations(qname)
                    .map(|def| def.name_span.start())
                    .min()
                    .unwrap_or(u32::MAX);
                (qname.clone(), earliest)
            })
            .collect();
        let rename_offsets: FxHashMap<String, u32> = a
            .result
            .renamed_commands
            .keys()
            .filter_map(|new_qname| {
                a.rename_offsets
                    .get(new_qname)
                    .map(|&off| (new_qname.clone(), off))
            })
            .collect();
        let stub_names = super::utils::scan_stub_command_names(&a.source);
        Self {
            non_proc_qnames,
            proc_offsets,
            rename_offsets,
            stub_names,
        }
    }

    /// Whether a call to `cmd_name` (resolved at `ns`, through the implicit
    /// `namespace path` `path`, at byte offset `call_off`) resolves to a
    /// user-declared proc, class, alias, rename target, ensemble, or stub,
    /// rather than falling through to the registry builtin that produced the
    /// candidate diagnostic.
    ///
    /// `enforce_order` gates **proc** and **rename** shadowing to a
    /// definition that lexically precedes the call: true for a top-level
    /// call site (the module body, a `namespace eval` body, or a
    /// conditional), which executes in source order during load, so a
    /// same-named definition appearing later in the file has not run yet. A
    /// proc-body call (`enforce_order == false`) runs only after the whole
    /// file has loaded, so any same-named definition anywhere in the file is
    /// already in effect. Classes / aliases / ensembles / stubs always exist
    /// by run time and are never order-gated.
    fn resolves_to_user(
        &self,
        cmd_name: &str,
        ns: &str,
        path: &[&str],
        enforce_order: bool,
        call_off: u32,
    ) -> bool {
        let bare = cmd_name.rsplit("::").next().unwrap_or(cmd_name);
        let candidates = qualify_candidates_with_path(ns, cmd_name, path);
        candidates.iter().any(|c| {
            self.non_proc_qnames.contains(c.as_str())
                || self
                    .proc_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
                || self
                    .rename_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
        }) || self.stub_names.contains(bare)
    }

    /// Whether **any** of an invocation's already-resolved command-resolution
    /// candidates names a command this document itself declares — a proc, a
    /// class, an `interp alias`, a static `rename` target, an ensemble
    /// namespace, or a `# tcl-lsp: stub` name.
    ///
    /// The order-gated sibling ([`Self::resolves_to_user`]) answers "which
    /// definition is in effect *at this call*", which is what a
    /// wrong-arguments verdict needs. This one answers the weaker question
    /// W120 asks — "does this file define the command at all" — for which
    /// order is irrelevant: `package require` is a statement about what the
    /// *file* needs loaded, and a file that defines the command needs nothing
    /// loaded to make the name exist, whatever line the definition is on.
    /// Deliberately permissive for the same reason the non-proc set is
    /// (issue #923 idx 11, verification pass: `proc ::argparse {args}`
    /// beside a call to it was told to `package require argparse`).
    pub(super) fn declares_any(&self, candidates: &[String], bare: &str) -> bool {
        candidates.iter().any(|c| {
            self.non_proc_qnames.contains(c.as_str())
                || self.proc_offsets.contains_key(c.as_str())
                || self.rename_offsets.contains_key(c.as_str())
        }) || self.stub_names.contains(bare)
    }
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
    /// **W002** — the command is disabled in the active dialect profile: it
    /// exists in the registry but not for the active dialect (e.g. `dict` under
    /// `tcl8.4`, added in 8.5).  Only a *literal* command head is checked — a
    /// `$obj` / `[cmd]` head is W307's concern.
    ///
    /// Queues the candidate onto [`Self::pending_disabled_commands`] rather
    /// than emitting inline: whether the call actually reaches the disabled
    /// builtin depends on same-file facts (a shadowing proc / class / `interp
    /// alias` / static `rename` / `namespace ensemble create`) that may not
    /// be fully known until the whole file (or, on the per-item path, every
    /// grafted body) has been walked — a forward-declared shadowing proc is
    /// the common case a same-pass check would miss entirely.
    /// [`Self::flush_disabled_command_diagnostics`] resolves every candidate
    /// post-walk using the same current-namespace-then-global resolution and
    /// top-level order gate as [`Self::flush_arity_diagnostics`].
    pub(in crate::analyser) fn emit_w002_disabled_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        // A dynamic command head (`$obj method`, `[lookup] arg`) is resolved at
        // runtime — W307 handles it, not W002.
        if matches!(
            cmd_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) {
            return;
        }
        let Some(registry) = self.registry else {
            return;
        };
        let bare = cmd_name.trim_start_matches(':');
        if bare.is_empty() {
            return;
        }
        let profile = tcl_dialect::DialectProfile::by_name(self.dialect());
        // EXISTS in the active dialect profile → fine.  UNKNOWN everywhere →
        // W123's concern.  Only DISALLOWED (exists in some dialect, not this
        // one) fires.  Resolution goes through the profile so the composed
        // (version|vendor) mask admits the dialect's embedded Tcl core and
        // the subtractive iRules disable list stays applied after the mask
        // query.  Existence must be checked *dialect-agnostically*: the
        // analyser registry only loads the active dialect, so `get(bare)`
        // misses an iRules command like `when`/`log`/`session` under
        // tcl8.6, so use the dialect-independent `known_in_any_dialect`.
        if profile.resolve_command(registry, bare).is_some() || !registry.known_in_any_dialect(bare)
        {
            return;
        }
        // An earlier *unconditional* user proc with this name shadows the
        // would-be-disabled built-in at the call site.
        let qualified = crate::naming::normalise_qualified_name(bare);
        if super::utils::proc_shadows_call(&self.result.all_procs, &qualified, cmd_tok.span.start())
        {
            return;
        }
        // Best-effort "available in: …" hint read straight from the
        // registry spec's own dialect gate. Only resolves when the spec is
        // among the packs this registry instance loaded regardless of
        // dialect (core Tcl / stdlib / tcllib / Tk / itcl / …, via
        // `CommandRegistry::build_default`); a command exclusive to a
        // *different* dialect family (e.g. an iRules-only name referenced
        // from a `tcl8.6` file) isn't loaded here, so `get` returns `None`
        // and the message falls back to the plain form — never wrong, just
        // sometimes less specific.
        let suffix = registry.get(bare).map_or(String::new(), |spec| {
            dialect_availability_suffix(spec.dialects)
        });
        let diag = super::types::Diagnostic {
            code: DiagCode::W002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is disabled in the active dialect profile{suffix}"),
            severity: Severity::Warning,
            fixes: Vec::new(),
        };
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        self.pending_disabled_commands
            .push((cmd_name.to_string(), ns, enforce_order, diag));
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
    ) -> Option<super::dispatch::CommandSignature> {
        if let Some(env) = self.body_scope_stack.last()
            && let Some(scoped) = env.command(cmd_name)
        {
            return Some(super::dispatch::signature_for_scoped_command(scoped));
        }
        let registry = self.registry?;
        super::dispatch::signature_for_command(registry, cmd_name, self.profile)
    }

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
    /// - a `{*}`-expanded subcommand word (`cmd {*}bogus …`) — its literal
    ///   text is not what ends up in the subcommand position at run time,
    /// - first-arg values containing ``$`` / ``[`` (dynamic
    ///   substitution — runtime-resolved),
    /// - empty arg lists (handled by the E001 emitter).
    ///
    /// When emission is warranted, includes a "did you mean…?"
    /// suffix using [`crate::text::suggest_similar`] over the
    /// known subcommand set (max 1 suggestion within the length-scaled
    /// edit-distance budget), and anchors the diagnostic at the subcommand token's
    /// *content* range only ([`subcommand_content_span`]) — not the command
    /// name — so the squiggle sits tightly on the one word that is actually
    /// wrong, matching the "did you mean" fix's replacement range.
    ///
    /// A verdict is not pushed directly: the genuinely-unknown-subcommand
    /// case is queued into [`Analyser::pending_arity`] — the same queue the
    /// sibling E002/E003 arity check and W004 use, since a wrong subcommand
    /// name is the same species of "call is malformed against the resolved
    /// registry signature" as a wrong argument count — and the
    /// subcommand-level-disabled-in-dialect case into
    /// [`Analyser::pending_disabled_commands`] (the same queue
    /// [`Self::emit_w002_disabled_command`] uses). Both are resolved
    /// post-walk through [`UserResolutionFacts`], the identical shadow
    /// computation every one of those checks shares. This covers the same
    /// tricky-Tcl surface: a same-file `proc string {…}`, an `oo::class` /
    /// snit / itcl class named `string`, an `interp alias` pointed at
    /// `string`, a `namespace ensemble create -command string`, a static
    /// `rename` target, or an inline `# tcl-lsp: stub string` all resolve
    /// the call to something the registry ensemble signature has nothing to
    /// do with, so none of them may draw a false "unknown subcommand".
    pub(in crate::analyser) fn emit_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        use super::dispatch::CommandSignature;
        use tcl_registry::prelude::DialectSet;

        if self.registry.is_none() {
            return;
        }
        let Some(first_arg) = args.first() else {
            // Empty arg list — E001 path; not in scope here.
            return;
        };
        // A `{*}`-expanded subcommand word splices its *elements* into the
        // subcommand position at run time, not its literal text.
        // `arg_expand_in` is parallel to the full argv (command name at
        // index 0); drop that slot so it lines up with `args`.
        let arg_expand: &[bool] = arg_expand_in.get(1..).unwrap_or(&[]);
        if arg_expand.first().copied().unwrap_or(false) {
            return;
        }
        // Dynamic-value subcommand position — can't resolve statically.
        if arg_tokens
            .first()
            .is_some_and(|tok| has_substitution(first_arg, tok))
        {
            return;
        }
        // Scope-aware resolution: an ensemble scoped command (`top`, `data`)
        // inside a `report::defstyle` body is checked against its scoped
        // subcommand set, so `top bogus` still draws W001. Tk
        // geometry/widget ensemble commands (`grid` / `pack` / `wm` / …)
        // are additionally recognised regardless of the active dialect — a
        // `.tcl` script may `package require Tk` at runtime, so W001 fires
        // on `grid bogus` under every dialect (the Tk-tagged fallback
        // below; Tk is a library over a Tcl base, not a profile).
        let tk_fallback = || {
            let registry = self.registry?;
            let spec = registry.get(cmd_name)?;
            spec.dialects
                .is_some_and(|d| d.intersects(DialectSet::TK))
                .then(|| super::dispatch::signature_for_command_any_dialect(registry, cmd_name))
                .flatten()
        };
        let Some(CommandSignature::WithSubcommands(sig)) = self
            .resolve_command_signature(cmd_name)
            .or_else(tk_fallback)
        else {
            // Not a registered ensemble — buffer as a widget-dispatch
            // candidate, resolved post-walk (issue #927).
            self.record_widget_dispatch_candidate(
                cmd_name,
                args,
                cmd_tok,
                arg_tokens,
                arg_expand_in,
            );
            return;
        };
        if sig.allow_unknown {
            return;
        }
        if shape_exempt_from_w001(&sig, first_arg) {
            return;
        }
        // Resolve the word through the shared registry abbreviation API
        // (#1231): a unique prefix (`string le` ⇒ `length`) is legal and must
        // not trip W001; an ambiguous one (`string l`) is a guaranteed
        // runtime error with its own diagnostic (W145) rather than an
        // "unknown subcommand" guess.
        match sig.resolve_word(first_arg) {
            tcl_registry::abbrev::KeywordMatch::Unique(_) => return,
            tcl_registry::abbrev::KeywordMatch::Ambiguous(candidates) => {
                let candidates: Vec<String> = candidates.iter().map(|s| (*s).to_string()).collect();
                self.emit_w145_ambiguous_abbreviation(
                    cmd_name,
                    first_arg,
                    &candidates,
                    cmd_tok,
                    arg_tokens,
                );
                return;
            }
            tcl_registry::abbrev::KeywordMatch::Unknown => {}
        }
        // The subcommand is unknown *in the active dialect* — before
        // reporting it as nonexistent, check whether it exists in some
        // *other* dialect (e.g. a Tcl 9.0-only subcommand under an 8.6
        // profile) and report disabled-in-dialect instead (issue #812).
        if self.check_disabled_in_other_dialect_subcommand(
            cmd_name, first_arg, cmd_tok, arg_tokens, scope_path,
        ) {
            return;
        }
        // A statically-declared ensemble's subcommand table can't reflect a
        // runtime `namespace ensemble configure <cmd> -map [dict replace
        // [namespace ensemble configure <cmd> -map] NAME impl]` patch (the
        // real tcllib `dicttool.tcl` idiom for `dict getnull`/`print`/
        // `is_dict`/…, issue #923 idx 105) — if a proc exists at this
        // ensemble's conventional implementation location for the written
        // subcommand name, treat it as known rather than reporting a false
        // "unknown subcommand". Registry-driven via
        // `CommandSpec::implementation_namespace`, so it applies to any
        // future ensemble that declares one, not just `dict`.
        if self.dynamic_ensemble_subcommand_known(cmd_name, first_arg)
            || self.statically_mapped_ensemble_subcommand_known(cmd_name, first_arg, scope_path)
        {
            return;
        }
        self.push_w001_unknown_subcommand(
            cmd_name, first_arg, &sig, cmd_tok, arg_tokens, scope_path,
        );
    }

    /// Queue the W001 verdict for a genuinely unknown subcommand word, with
    /// its "did you mean…?" suffix and matching quick fix.
    ///
    /// Split out of [`Self::emit_w001_unknown_subcommand`], which is all
    /// abstention checks up to this point.
    fn push_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        first_arg: &str,
        sig: &super::dispatch::SubcommandSig,
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        scope_path: &[usize],
    ) {
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        let mut message = format!("Unknown subcommand '{first_arg}' for '{cmd_name}'");
        let candidates: Vec<&str> = sig.subcommands.keys().map(String::as_str).collect();
        let suggestions = crate::text::suggest_similar(
            first_arg,
            candidates.iter().copied(),
            1,
            crate::text::scaled_max_distance(first_arg),
        );
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        // Anchor the squiggle at the subcommand token's content range only —
        // not the command name — so it sits tightly on the one word that is
        // actually wrong; the "did you mean" fix targets the identical range.
        let span = match arg_tokens.first() {
            Some(sub_tok) => subcommand_content_span(*sub_tok),
            None => cmd_tok.span,
        };
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            fixes.push(super::types::CodeFix {
                span,
                new_text: (*best).to_string(),
                description: format!("Replace with '{best}'"),
                // W001: an edit-distance guess at the intended subcommand.
                safety: crate::irules_checks::FixSafety::RequiresReview,
            });
        }
        self.pending_arity.push((
            cmd_name.to_string(),
            ns,
            enforce_order,
            super::types::Diagnostic {
                code: DiagCode::W001,
                span,
                message,
                severity: Severity::Warning,
                fixes,
            },
        ));
    }

    /// **W145.** The subcommand word prefixes more than one subcommand, so
    /// the call is a guaranteed runtime error (`string l` →
    /// `unknown or ambiguous subcommand "l"` in real tclsh).
    ///
    /// The message quotes the *matching* candidates rather than the whole
    /// table — that is what the user needs to disambiguate — and offers one
    /// `RequiresReview` quick fix per candidate, since the tool cannot know
    /// which was meant.
    ///
    /// Abstains when the file configured this ensemble with
    /// `namespace ensemble … -prefixes 0`: prefix matching is off there, so
    /// the word is a plain unknown subcommand and W001 owns it.  A strict
    /// registry table never produces `Ambiguous` in the first place.
    fn emit_w145_ambiguous_abbreviation(
        &mut self,
        cmd_name: &str,
        word: &str,
        candidates: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if self.prefixless_ensembles.contains(cmd_name)
            || self
                .prefixless_ensembles
                .contains(&format!("::{}", cmd_name.trim_start_matches(':')))
        {
            return;
        }
        let span = match arg_tokens.first() {
            Some(sub_tok) => subcommand_content_span(*sub_tok),
            None => cmd_tok.span,
        };
        let listed = candidates
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let fixes: Vec<super::types::CodeFix> = candidates
            .iter()
            .map(|candidate| super::types::CodeFix {
                span,
                new_text: candidate.clone(),
                description: format!("Expand to '{cmd_name} {candidate}'"),
                // The tool cannot know which candidate was meant, so this is
                // never auto-applied by Fix All.
                safety: crate::irules_checks::FixSafety::RequiresReview,
            })
            .collect();
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W145,
            span,
            message: format!("Ambiguous abbreviation '{word}' for '{cmd_name}': matches {listed}."),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Whether `cmd_name subcommand_name …` is a call into an ensemble
    /// whose `-map` may have been reconfigured at runtime to add
    /// `subcommand_name`, pointing at a proc this file defines at the
    /// ensemble's conventional implementation location — the real tcllib
    /// `dicttool.tcl` idiom: `proc ::tcl::dict::getnull {d args} {...} ;
    /// namespace ensemble configure dict -map [dict replace [namespace
    /// ensemble configure dict -map] getnull ::tcl::dict::getnull]` (issue
    /// #923 idx 105).
    ///
    /// Deliberately does **not** require observing an actual `namespace
    /// ensemble configure -map` call in the file — only that
    /// [`tcl_registry::CommandSpec::implementation_namespace`] is set (so
    /// this stays inert for every other ensemble, `string`/`array`/…, whose
    /// subcommands are C `switch` arms with no such backing namespace) and
    /// that a same-named proc exists there. This is a deliberate,
    /// conservative-suppression trade: a proc defined at
    /// `<implementation_namespace>::<subcommand_name>` but never actually
    /// spliced into the ensemble's map (a genuine "unknown subcommand"
    /// error at run time) is wrongly suppressed here — the same
    /// risk-tolerance precedent as W123's file-wide "any `package require`
    /// suppresses every unknown-command diagnostic" abstention, and much
    /// narrower in scope (this ensemble, this subcommand name, a proc
    /// already known to exist).
    fn dynamic_ensemble_subcommand_known(&self, cmd_name: &str, subcommand_name: &str) -> bool {
        let Some(registry) = self.registry else {
            return false;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return false;
        };
        let Some(ns) = spec.implementation_namespace else {
            return false;
        };
        self.result
            .all_procs
            .contains_key(&format!("{ns}::{subcommand_name}"))
    }

    /// Whether `cmd_name subcommand_name` is a call into an ensemble whose
    /// `-map`/`-subcommands` *this file itself* literally, statically
    /// extends via `namespace ensemble create`/`configure`
    /// ([`super::super::handlers::Analyser::handle_namespace_ensemble`],
    /// which populates [`AnalysisResult::ensemble_subcommand_targets`]) —
    /// the real `tk/library/systray.tcl` idiom (issue #923 idx 84):
    /// `namespace ensemble configure tk -map [dict merge [namespace
    /// ensemble configure tk -map] {systray ::tk::systray sysnotify
    /// ::tk::sysnotify::sysnotify}]`, which splices `systray`/`sysnotify`
    /// onto the pre-existing, registry-builtin `tk` ensemble.
    ///
    /// Complements [`Self::dynamic_ensemble_subcommand_known`] (which needs
    /// [`tcl_registry::CommandSpec::implementation_namespace`] — unset for
    /// `tk`, since `tk systray`'s own genuinely 2-level nested-ensemble
    /// shape doesn't fit that field's single-hop `<ns>::<subcommand>`
    /// convention anyway) with the general, ensemble-agnostic fact this
    /// file's own analysis already recorded, rather than a `tk`-specific
    /// escape hatch.
    ///
    /// [`AnalysisResult::ensemble_subcommand_targets`]: super::super::types::AnalysisResult::ensemble_subcommand_targets
    fn statically_mapped_ensemble_subcommand_known(
        &self,
        cmd_name: &str,
        subcommand_name: &str,
        scope_path: &[usize],
    ) -> bool {
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .any(|cand| {
                self.result
                    .ensemble_subcommand_targets
                    .get(&cand)
                    .is_some_and(|subs| subs.contains_key(subcommand_name))
            })
    }

    /// `first_arg` is unknown in the active dialect profile but exists as a
    /// real subcommand in some *other* dialect (e.g. a Tcl 9.0-only
    /// subcommand checked under an 8.6 profile, issue #812) — the
    /// subcommand-level analogue of the whole-command W002 check
    /// (`emit_w002_disabled_command`): it EXISTS, just not here, so report
    /// disabled-in-dialect rather than "Unknown subcommand" with a
    /// misleading spelling suggestion. Deferred like the whole-command
    /// form (buffered into `pending_disabled_commands`, not emitted
    /// inline): whether `cmd_name` itself resolves to the registry
    /// ensemble, rather than a same-file proc/alias/rename/class/ensemble
    /// that shadows it, can depend on facts not yet known mid-walk.
    /// Returns whether it handled (and buffered) the site — the caller
    /// should return without falling through to W001 either way.
    /// Split out of [`Self::emit_w001_unknown_subcommand`] to keep that
    /// function under the line-count lint.
    fn check_disabled_in_other_dialect_subcommand(
        &mut self,
        cmd_name: &str,
        first_arg: &str,
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        scope_path: &[usize],
    ) -> bool {
        use super::dispatch::{CommandSignature, signature_for_command_any_dialect};

        let Some(registry) = self.registry else {
            return false;
        };
        let Some(CommandSignature::WithSubcommands(any_sig)) =
            signature_for_command_any_dialect(registry, cmd_name)
        else {
            return false;
        };
        if !any_sig.is_known(first_arg) {
            return false;
        }
        let span = match arg_tokens.first() {
            Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
            None => cmd_tok.span,
        };
        // Best-effort "available in: …" hint from the registry's own
        // dialect gate on the matching `SubCommand` entry (falling back to
        // the parent command's, the same inheritance
        // `emit_w004_dialect_invalid_option` uses). An abbreviated
        // `first_arg` that doesn't literally match a `SubCommand.name` just
        // yields no hint — the base message stays accurate either way.
        let suffix = registry.get(cmd_name).map_or(String::new(), |spec| {
            spec.subcommands
                .iter()
                .find(|s| s.name == first_arg)
                .map_or(String::new(), |sub| {
                    dialect_availability_suffix(sub.dialects.or(spec.dialects))
                })
        });
        let diag = super::types::Diagnostic {
            code: DiagCode::W002,
            span,
            message: format!(
                "'{cmd_name} {first_arg}' is disabled in the active dialect profile{suffix}"
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        };
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        self.pending_disabled_commands
            .push((cmd_name.to_string(), ns, enforce_order, diag));
        true
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
        // name ::ns ?args?`) and `next`/`nextto` call-site arity — see
        // [`Self::queue_tcloo_arity_candidates`].
        self.queue_tcloo_arity_candidates(
            cmd_name,
            &ArityWords {
                args,
                arg_tokens,
                arg_expand,
                cmd_tok,
            },
            scope_path,
        );

        // `apply {{params} body} ?arg ...?` — a direct call to an inline
        // lambda. Unlike the two candidates above, the lambda's own arity
        // is fully knowable at the call site (no forward reference, no
        // cross-item merge to wait for), so this checks synchronously
        // rather than through the pending/flush queue.
        self.emit_apply_lambda_arity(cmd_name, args, arg_tokens, arg_expand, cmd_tok, scope_path);

        // Scope-aware: a head inside a scoped command environment resolves to
        // its scoped signature (`top set …`, `columns`), everything else to
        // the global registry via the active profile.
        match self.resolve_command_signature(cmd_name) {
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
        let excess = excess_positional_span(
            arg_tokens,
            arg_expand,
            positional_start,
            usize::from(sig.arity.max),
            &self.source,
            widen_token_end(cmd_tok, &self.source),
        );
        if let Some(diag) = arity_verdict(
            display_name,
            sig.arity,
            nargs_min,
            positional_any_expand,
            full_span,
            excess,
            sig.synopsis,
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
        // The head must actually resolve to `apply` — registry hook
        // resolution, not a name literal, decides that (the same
        // [`AnalyserHookId::Apply`] gate the handler dispatch uses).
        // Without the gate, any command whose first argument is a braced
        // literal (`proc {a b} {…}`) would be arity-checked as a lambda.
        if !matches!(
            self.resolve_analyser_hook(cmd_name, args),
            Some(tcl_registry::hooks::AnalyserHookId::Apply)
        ) {
            return;
        }
        let Some(elements) = self.parse_apply_lambda_elements(args, arg_tokens) else {
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
        // Positional args (and so the surplus run) start after the lambda
        // literal at index 1.
        let excess = excess_positional_span(
            arg_tokens,
            arg_expand,
            1,
            usize::from(arity.max),
            &self.source,
            widen_token_end(cmd_tok, &self.source),
        );
        if let Some(diag) = arity_verdict(
            "apply",
            arity,
            nargs_min,
            positional_any_expand,
            full_span,
            excess,
            None,
        ) {
            let ns = self.command_resolution_namespace(scope_path);
            let enforce_order = !self.scope_path_in_proc_body(scope_path);
            self.pending_arity
                .push((cmd_name.to_string(), ns, enforce_order, diag));
        }
    }

    /// Post-walk flush of the [`Self::pending_disabled_commands`] candidates
    /// collected by [`Self::emit_w002_disabled_command`] and the subcommand
    /// form embedded in [`Self::emit_w001_unknown_subcommand`].
    ///
    /// Runs after the command walk completes, when `all_procs`,
    /// `all_classes`, `command_aliases`, `renamed_commands` and
    /// `ensemble_namespaces` are fully populated (post cross-item merge on
    /// the per-item path). A candidate is dropped exactly when
    /// [`UserResolutionFacts::resolves_to_user`] says the call resolves to a
    /// user definition rather than the builtin the candidate warns about —
    /// the identical resolution rule [`Self::flush_arity_diagnostics`] uses
    /// to suppress a builtin-arity mismatch, so e.g. a namespace-scoped `proc
    /// ::ns::dict {...}` correctly suppresses a `dict` call inside `::ns`,
    /// and an `interp alias {} dict {} …` / `rename myimpl dict` /
    /// `oo::class create dict {…}` established anywhere in the file
    /// (unconditionally for aliases/classes/ensembles/stubs; only when
    /// lexically preceding a top-level call for a proc/rename target)
    /// likewise suppresses it.
    ///
    /// Idempotent: drains `pending_disabled_commands`, so a second call is a
    /// no-op.
    pub(in crate::analyser) fn flush_disabled_command_diagnostics(&mut self) {
        if self.pending_disabled_commands.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);
        let pending = std::mem::take(&mut self.pending_disabled_commands);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// Whether `name` resolves to a registry command that only exists once a
    /// `required_package` / `tcllib_package` is loaded, and this document
    /// neither `package require`s nor `package provide`s it — with the
    /// profile's ambient packages excluded, exactly as
    /// [`Analyser::is_package_gated_non_ambient`] and the W120 emitter do.
    ///
    /// Such a call has **no proven identity**: the registry's `CommandSpec`
    /// describes the package's command, but nothing in this document says
    /// that package is what the name will resolve to at run time — which is
    /// the very thing W120 is emitted to say. Asserting a hard arity *error*
    /// against that spec while simultaneously warning that the package is not
    /// loaded is self-contradictory, and it is wrong whenever the name is
    /// actually supplied by something else: a proc defined in a file this one
    /// `source`s, an `interp alias` set up by its host application.
    ///
    /// The real corpus case (issue #923 idx 11, verification pass) is
    /// georgtree/argparse's own `proc ::argparse {args}`: a sibling file that
    /// `source`s it and calls `argparse` with no arguments — which real tclsh
    /// 9.0.4 / 8.6.16 both run happily, printing `case0` — was told
    /// `Too few arguments for 'argparse': expected at least 1, got 0`,
    /// resolved against the *package's* spec that this workspace never loads.
    /// Whole-file `source`-graph knowledge is not available here (the analyser
    /// is single-file by construction, see
    /// [`Analyser::emit_missing_package_require_diagnostics`]), so the sound
    /// answer is to abstain and let W120 carry the (retractable, warning-level)
    /// report.
    ///
    /// The true positive is untouched: a document that *does* `package
    /// require argparse` has proven the identity, W120 does not fire, and a
    /// bad-arity call to it is still an error.
    fn spec_is_an_unloaded_package_command(&self, name: &str) -> bool {
        use tcl_registry::ProfileQueries;
        let registry = tcl_registry::cache::registry_for_profile(self.profile);
        let Some(pkg) = self
            .profile
            .resolve_command(registry, name)
            .and_then(tcl_registry::CommandSpec::owning_package)
        else {
            return false;
        };
        if self.profile.is_ambient_package(pkg) {
            return false;
        }
        !self
            .result
            .package_requires
            .iter()
            .map(|pr| pr.name.as_str())
            .chain(
                self.result
                    .package_provides
                    .iter()
                    .map(|pp| pp.name.as_str()),
            )
            .any(|imported| imported == pkg)
    }

    /// Post-walk flush of the [`Self::pending_arity`] / [`Self::pending_user_call_arity`]
    /// candidates collected by [`Self::emit_arity_diagnostics`] and
    /// [`Self::queue_user_call_arity_candidate`].
    ///
    /// The [`Self::pending_arity`] half drops a candidate exactly when
    /// [`UserResolutionFacts::resolves_to_user`] says the call **resolves
    /// to** a user definition rather than the builtin whose registry arity
    /// produced it — resolution follows Tcl's rule for unqualified commands
    /// (the call-site namespace, then global `::`), using the namespace
    /// captured at emit time.  So `proc ::ns::close {...}` suppresses a
    /// `close` call inside `::ns` (and a qualified `::ns::close ...`), but a
    /// `close` call in another namespace still resolves to the builtin and
    /// is checked.  Document-global declarations — inline `# tcl-lsp:
    /// stub`s — suppress by bare name regardless of namespace.
    ///
    /// Suppression by a shadowing **proc** or **rename target** also honours
    /// definition reachability: a top-level call (one whose `enforce_order`
    /// flag is set — module body, `namespace eval` body, or a conditional)
    /// is silenced only when the definition lexically precedes it, since
    /// top-level commands run in source order during load (so a `close x y
    /// z` *before* a later `proc close` still reaches the builtin).
    /// Proc-body calls run after load and are not order-gated.  Classes /
    /// aliases / ensembles / stubs always exist at run time and are never
    /// order-gated.  (Excluding *conditionally* defined procs would need the
    /// CFG dominator model, which is not modelled here.)
    ///
    /// Idempotent: drains `pending_arity` and `pending_user_call_arity`,
    /// so a second call is a no-op.
    ///
    /// `pending_arity` carries E002/E003 arity candidates, W004
    /// (dialect-invalid-option) candidates, and W001 (unknown-subcommand)
    /// candidates (see its field doc); the *shadowing* suppression never
    /// inspects `diag.code`, so all three share it with no special-casing.
    /// The package-loaded gate below is the one exception, and it is scoped
    /// to the arity verdicts because only they assert against the resolved
    /// spec's own argument counts — see
    /// [`Self::spec_is_an_unloaded_package_command`].
    pub fn flush_arity_diagnostics(&mut self) {
        if self.pending_arity.is_empty() && self.pending_user_call_arity.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);

        let pending = std::mem::take(&mut self.pending_arity);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            // Arity verdicts only — the *count* claim is the one that needs
            // the spec to be the command actually being called. W001
            // (unknown subcommand) and W004 (dialect-invalid option) share
            // this queue but are claims about the *word the user wrote*
            // against a name the registry knows, and they stay useful for a
            // package the file has not required yet (`wm bogus` is still a
            // typo whether or not `package require Tk` is present) — the two
            // TP tests in `fp::sty` pin exactly that.
            if matches!(diag.code, DiagCode::E002 | DiagCode::E003 | DiagCode::E005)
                && self.spec_is_an_unloaded_package_command(&cmd_name)
            {
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
            if facts.stub_names.contains(bare) {
                continue;
            }
            let Some(arity) = self.resolve_indirect_call_target(&cand) else {
                continue;
            };
            // The surplus-run anchor: only computable now that the resolved
            // arity's `max` is known. `arg_spans` is empty under `{*}`
            // expansion, so `.get(max)` naturally abstains there.
            let excess = (!arity.is_unlimited())
                .then(|| {
                    let max = usize::from(arity.max);
                    let first = cand.arg_spans.get(max)?;
                    let last = cand.arg_spans.last()?;
                    let delete_from = match max.checked_sub(1).and_then(|i| cand.arg_spans.get(i)) {
                        Some(prev) => prev.end(),
                        None => cand.head_end,
                    };
                    Some(ExcessArgs {
                        span: tcl_lexer::Span::new(first.start(), last.end()),
                        delete_from,
                    })
                })
                .flatten();
            if let Some(diag) = arity_verdict(
                &cand.cmd_name,
                arity,
                cand.nargs_min,
                cand.positional_any_expand,
                cand.full_span,
                excess,
                None,
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
                if !is_tcloo_metaclass(self.registry, &cd.metaclass) {
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
                    // No explicit constructor anywhere in the MRO — TclOO's
                    // inherited default constructor accepts any argument
                    // count, so `new` is never checked here. The form's own
                    // mandatory leading words are a separate requirement
                    // enforced by the dispatcher itself, independent of the
                    // constructor: `oo::class create Foo {}; Foo create` still
                    // raises "wrong # args" (confirmed against tclsh 9.0.4)
                    // even though nothing constrains the constructor args that
                    // may follow the name. Reuses `extra_leading_words` so
                    // `create` (1 word) and `createWithNamespace` (2) are both
                    // covered by the same rule.
                    let extra = cand.form.extra_leading_words();
                    if extra > 0 {
                        let arity = bump_arity(Arity::at_least(0), extra);
                        let display_name = format!("{} {}", cand.class_name, cand.form.as_str());
                        if let Some(diag) = arity_verdict(
                            &display_name,
                            arity,
                            cand.nargs_min,
                            cand.positional_any_expand,
                            cand.full_span,
                            None,
                            None,
                        ) {
                            diags.push(diag);
                        }
                    }
                    continue;
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
                let arity = ctor.arity();
                let arity = bump_arity(arity, cand.form.extra_leading_words());
                let display_name = format!("{} {}", cand.class_name, cand.form.as_str());
                if let Some(diag) = arity_verdict(
                    &display_name,
                    arity,
                    cand.nargs_min,
                    cand.positional_any_expand,
                    cand.full_span,
                    None,
                    None,
                ) {
                    diags.push(diag);
                }
            }
        }
        self.result.diagnostics.extend(diags);
    }

    /// Idempotent: drains `pending_next_arity`, so a second call is a no-op.
    ///
    /// Resolves each queued `next` / `nextto` candidate against
    /// `all_classes` (fully populated post-walk) via
    /// [`super::class_hierarchy::ClassHierarchy::next_provider`], treating
    /// the enclosing method's own class as the receiver's MRO — the same
    /// "same instance" simplification [`super::var_command::Analyser::method_arity_diagnostic`]
    /// already makes for `$obj method` dispatch. This is exact for single
    /// inheritance (confirmed against tclsh 9.0.4: a linear chain's MRO
    /// tail from any ancestor onward is identical regardless of which
    /// subclass views it); with mixins or multiple inheritance a
    /// subclass's C3-style linearisation can reorder ancestors, so a
    /// `next` inherited unchanged into such a subclass could in principle
    /// resolve to a different provider there than this check assumes — a
    /// known, narrow imprecision in the same spirit as this module's other
    /// documented gaps, not a soundness hole for the common case.
    ///
    /// A `nextto` target that isn't a locally-known class, or a `next`
    /// past the end of the MRO chain (a real `next` there is itself a
    /// runtime error — Tcl raises "no next" — not an arity mismatch), is
    /// silently dropped. A forwarded method (`forward` — see
    /// [`super::var_command::Analyser::method_arity_diagnostic`]'s own hop
    /// chase) is not resolved further here; forwarding to `next`/`nextto`
    /// is vanishingly rare and left unchecked rather than duplicating that
    /// hop logic.
    pub fn flush_next_arity_diagnostics(&mut self) {
        if self.pending_next_arity.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_next_arity);
        let mut diags: Vec<super::types::Diagnostic> = Vec::new();
        {
            let hierarchy = self.result.class_hierarchy();
            for cand in &pending {
                let start_from = match &cand.target_class {
                    Some(name) => {
                        let candidates = qualify_candidates(&cand.ns, name);
                        let Some(qn) = candidates
                            .into_iter()
                            .find(|c| self.result.all_classes.contains_key(c.as_str()))
                        else {
                            continue; // dynamic / cross-file / unresolvable target class
                        };
                        Some(qn)
                    }
                    None => None,
                };
                // `member_next_provider` routes the two nameless slots to
                // their own providers, so a `next` inside a `constructor`
                // (or `destructor`) is arity-checked exactly like one
                // inside a plain `method` — it used to be dropped
                // silently, letting a real `wrong # args` crash into a
                // superclass constructor go undiagnosed (issue #923 idx
                // 37).
                let Some(provider) = hierarchy.member_next_provider(
                    &cand.class_qualified,
                    &cand.method_name,
                    &cand.class_qualified,
                    start_from.as_deref(),
                    &self.source,
                ) else {
                    continue;
                };
                let Some(method_def) =
                    self.result.all_classes.get(provider).and_then(|cd| {
                        super::class_hierarchy::class_member_def(cd, &cand.method_name)
                    })
                else {
                    continue;
                };
                if method_def.kind == "forward" {
                    continue;
                }
                let arity = method_def.arity();
                if let Some(diag) = arity_verdict(
                    &cand.display_name,
                    arity,
                    cand.nargs_min,
                    cand.positional_any_expand,
                    cand.full_span,
                    None,
                    None,
                ) {
                    diags.push(diag);
                }
            }
        }
        self.result.diagnostics.extend(diags);
    }

    /// Queue the `TclOO`-specific arity candidates a call may trigger,
    /// alongside the ordinary same-file/registry checks
    /// [`Self::emit_arity_diagnostics`] always runs: a constructor call
    /// (`ClassName new ?args?` / `ClassName create name ?args?` /
    /// `ClassName createWithNamespace name ::ns ?args?`, queued
    /// unconditionally whenever the first word is literally one of those
    /// three keywords) and a `next`/`nextto` call (queued whenever the
    /// registry marks the command [`tcl_registry::Traits::TCLOO_NEXT_CHAIN`]).
    /// Both are resolved post-walk, once `all_classes` is fully
    /// populated — see [`Self::flush_ctor_arity_diagnostics`] /
    /// [`Self::flush_next_arity_diagnostics`]. Split out purely to keep
    /// `emit_arity_diagnostics` under the line-count lint.
    fn queue_tcloo_arity_candidates(
        &mut self,
        cmd_name: &str,
        words: &ArityWords<'_>,
        scope_path: &[usize],
    ) {
        if matches!(
            words.args.first().map(String::as_str),
            Some("new" | "create" | "createWithNamespace")
        ) {
            self.queue_ctor_arity_candidate(cmd_name, words, scope_path);
        }
        if self.registry.is_some_and(|r| {
            r.get(cmd_name)
                .is_some_and(|sig| sig.traits.contains(tcl_registry::Traits::TCLOO_NEXT_CHAIN))
        }) {
            self.queue_next_arity_candidate(cmd_name, words, scope_path);
        }
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
        // Argument-word spans for the flush-time E003 surplus anchor —
        // omitted under `{*}` expansion, where the surplus run is ambiguous.
        let arg_spans: Vec<tcl_lexer::Span> = if positional_any_expand {
            Vec::new()
        } else {
            arg_tokens
                .iter()
                .map(|t| widened_word_span(*t, &self.source))
                .collect()
        };
        self.pending_user_call_arity.push(PendingUserCallArity {
            cmd_name: cmd_name.to_string(),
            ns: self.command_resolution_namespace(scope_path),
            enforce_order: !self.scope_path_in_proc_body(scope_path),
            call_off: cmd_tok.span.start(),
            full_span,
            nargs_min,
            positional_any_expand,
            arg_spans,
            head_end: widen_token_end(cmd_tok, &self.source),
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

    /// Queue a `TclOO` `next` / `nextto` call-site arity candidate.
    ///
    /// The callee is never named at the call site — it's derived from
    /// *where* the call sits ([`Analyser::current_method_context`]).
    /// Silently drops the candidate when the call isn't lexically inside
    /// a method body: a `next`/`nextto` there is a runtime error in Tcl
    /// itself ("next may only be called from inside a method"), not an
    /// arity mismatch, and no other diagnostic here models that shape.
    ///
    /// `nextto`'s explicit target-class word (marked [`ArgRole::Name`] at
    /// index 0 in the registry — see `tcl-registry/src/commands/tcl/nextto.rs`)
    /// is read directly off `words.args`, structurally, never by
    /// comparing `cmd_name` against the literal string `"nextto"`.
    fn queue_next_arity_candidate(
        &mut self,
        cmd_name: &str,
        words: &ArityWords<'_>,
        scope_path: &[usize],
    ) {
        use tcl_registry::ArgRole;
        let Some((class_qualified, method_name)) = self.current_method_context(scope_path) else {
            return;
        };
        let ArityWords {
            args,
            arg_tokens,
            arg_expand,
            cmd_tok,
        } = *words;
        let has_target_class = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .is_some_and(|sig| {
                sig.arg_roles
                    .iter()
                    .any(|&(i, role)| i == 0 && role == ArgRole::Name)
            });
        let target_class = has_target_class.then(|| args.first().cloned()).flatten();
        // A `nextto` with no target word at all has nothing to resolve —
        // the registry's own `Arity::at_least(1)` already reports it.
        if has_target_class && target_class.is_none() {
            return;
        }
        let start = usize::from(has_target_class);
        let (nargs_min, positional_any_expand) = count_positionals(args, arg_expand, start);
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        self.pending_next_arity
            .push(super::types::PendingNextArity {
                class_qualified,
                method_name,
                target_class,
                ns: self.command_resolution_namespace(scope_path),
                display_name: cmd_name.to_string(),
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

    /// Whether `name`'s most recent recorded deletion supersedes a
    /// specific fact (a proc definition, a `rename` target, or an
    /// `interp alias` target) that was itself established at
    /// `fact_off`.
    ///
    /// `deleted_commands` holds only the *last* deletion offset seen for
    /// `name` during the walk (a later `HashMap` insert overwrites an
    /// earlier one) — but since the walk visits top-level statements in
    /// source order, "last inserted" and "highest offset" coincide, so
    /// that single stored offset is always the most recent deletion.
    /// Comparing it against `fact_off` (not just against the call site,
    /// as [`Self::fact_in_effect`] alone would) is what makes a
    /// re-establishment after a deletion resolve correctly: a name
    /// deleted at offset 7 and then given a fresh `proc`/`rename`/`interp
    /// alias` at offset 50 is live again for any call after 50, because
    /// the deletion (7) predates the fact (50) — only a deletion whose
    /// offset falls *between* the fact and the call still shadows it
    /// (confirmed against tclsh 9.0.4: `proc p {} {}`, `rename p {}`,
    /// `proc p {a b} {}`, `p 1 2` succeeds — the second `proc` overrides
    /// the deletion).
    ///
    /// A deletion recorded *inside* a proc/class/method body is itself
    /// conditional — it executes only if and when that body is ever
    /// invoked, which the textual load-order gate can't know — so it
    /// never supersedes anything (issue #1007: confirmed against tclsh
    /// 8.6.14 that `proc p {a b} {}`, `proc maybeDelete {} { rename p {}
    /// }`, `p 1 2 3` still fails "wrong # args" against `p`'s original
    /// 2-arg signature, since `maybeDelete` is never called and the
    /// `rename` never runs). Mirrors the equivalent guard
    /// [`Self::fact_live_for_call`] applies for the same question in the
    /// W123 pass (see issue #973).
    fn fact_superseded_by_deletion(
        &self,
        name: &str,
        fact_off: u32,
        cand: &PendingUserCallArity,
    ) -> bool {
        self.deleted_commands.get(name).is_some_and(|&del_off| {
            del_off > fact_off
                && !self.result.offset_is_inside_any_definition_body(del_off)
                && Self::fact_in_effect(cand, del_off)
        })
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
    /// (`deleted_commands`) is skipped as a proc match only when the
    /// deletion postdates *that specific fact's* own offset (see
    /// [`Self::fact_superseded_by_deletion`]) — a fact re-established
    /// after its deletion is live again (see
    /// [`crate::analyser::handlers::Analyser::handle_rename`]).
    /// `interp alias`'s prepended arguments shift the eventual arity
    /// down (real partial application, confirmed against tclsh 9.0.4);
    /// chained aliases/renames accumulate the shift transitively.
    /// Hop-limited as a defensive guard against a self-referential
    /// rename/alias cycle (never legitimate Tcl). Reaching a registry
    /// builtin only counts once at least one hop has happened — a
    /// *direct* hit on a builtin name is [`Self::check_simple_arity`]'s
    /// job, not this one's.
    /// The declaration of `qualified` that `cand`'s call actually dispatches
    /// — the latest one written before it under the top-level order gate,
    /// the last in the file inside a body.
    ///
    /// A qualified name declared twice is two definitions with two parameter
    /// lists, and `all_procs` only ever keeps the second; asking it alone made
    /// a call *between* the two unresolvable (the sole surviving definition
    /// fails the order gate), so a genuine `wrong # args` against the first
    /// definition went unreported. Oracle (tclsh 8.6.16 and 9.0.4): with
    /// `proc p {} {…}`, `p a b` between the declarations fails `wrong # args:
    /// should be "p"` — against the *first* signature, whatever the later one
    /// says (issue #923 idx 45).
    fn proc_definition_reached_by<'a>(
        &'a self,
        qualified: &str,
        cand: &PendingUserCallArity,
    ) -> Option<&'a super::super::types::ProcDef> {
        if !cand.enforce_order {
            return self.result.all_procs.get(qualified);
        }
        self.result
            .proc_declarations(qualified)
            .rfind(|def| Self::fact_in_effect(cand, def.name_span.start()))
    }

    fn resolve_indirect_call_target(&self, cand: &PendingUserCallArity) -> Option<Arity> {
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
                let Some(def) = self.proc_definition_reached_by(c, cand) else {
                    continue;
                };
                let proc_off = def.name_span.start();
                // A deletion recorded for `c` only shadows *this* proc
                // definition when it postdates `proc_off` — a re-`proc`
                // after a `rename c {}` supersedes the deletion (see
                // `fact_superseded_by_deletion`). Skipped entirely while
                // chasing a rename hop backward: `OLD` being deleted is
                // precisely what freed it up to serve as this rename's
                // source, never a reason to reject it.
                if !via_rename_hop && self.fact_superseded_by_deletion(c, proc_off, cand) {
                    continue;
                }
                let arity = def.arity();
                return Some(shift_arity(arity, prepended_total));
            }
            if let Some(old) = candidates.iter().find_map(|c| {
                let old = self.renamed_commands.get(c)?;
                let off = *self.rename_offsets.get(c)?;
                if !Self::fact_in_effect(cand, off) {
                    return None;
                }
                // The rename that established `c` (the target name) can
                // itself be superseded by a later deletion of `c` — but
                // only when `c` wasn't just reached via a rename hop
                // (`via_rename_hop`): a chain like `rename a b; rename b
                // c` records `b` as "deleted" by the *second* rename as a
                // pure side effect of moving it onward, not because the
                // first rename's `b -> a` mapping stopped holding — that
                // deletion is exactly what freed `b` up to serve as this
                // hop's source, same convention as the proc branch above.
                // An unrelated, later `rename c {}` on the name we have
                // NOT yet hopped through must still shadow it, though
                // (`same_file_call_to_renamed_away_name_does_not_false_positive`'s
                // shape one level up the chain).
                if !via_rename_hop && self.fact_superseded_by_deletion(c, off, cand) {
                    return None;
                }
                Some(old)
            }) {
                cur.clone_from(old);
                hopped = true;
                via_rename_hop = true;
                continue;
            }
            if let Some((target, prepended)) = candidates.iter().find_map(|c| {
                // An alias is re-resolved by name on every call (never
                // chased back through a rename the way a proc's original
                // definition is), so its own establishing offset — not a
                // blanket per-name check — decides whether a later
                // deletion of `c` shadows it. Exempted when `c` was just
                // reached via a rename hop, for the same reason as the
                // rename branch above: `rename a b` then chasing `b` back
                // to alias `a` must not be defeated by `b`'s own deletion
                // record (a side effect of that very rename).
                let alias = self.command_aliases.get(c)?;
                let off = *self.alias_offsets.get(c)?;
                if !Self::fact_in_effect(cand, off) {
                    return None;
                }
                if !via_rename_hop && self.fact_superseded_by_deletion(c, off, cand) {
                    return None;
                }
                Some(alias)
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
                    && let Some(sig) = self.registry.and_then(|r| r.get(bare))
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

    /// **W142.** Runs a resolved command's [`tcl_registry::CommandSpec::context_gate`]
    /// — a restriction keyed on lexical/dispatch context rather than
    /// argument shape (iRules `return`: bare-only directly inside a `when
    /// EVENT { … }` body, full syntax inside any `proc`). `self.current_event`
    /// is explicitly cleared for the duration of a `proc` body walk
    /// (`handle_proc_command`) — a proc is a new call frame, entered only
    /// when later called, never inline — so it reads `None` there even when
    /// the `proc` statement itself sits lexically inside a `when` body
    /// (itself a separate, independently-flagged `Irule5006` placement
    /// error). Passing it straight through is enough to distinguish the two
    /// contexts.
    pub(in crate::analyser) fn emit_w142_context_gate(
        &mut self,
        gate: tcl_registry::ContextGate,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
    ) {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(message) = gate(&arg_strs, self.current_event.is_some()) else {
            return;
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W142,
            span: cmd_tok.span,
            message: message.to_string(),
            severity: Severity::Warning,
            fixes: Vec::new(),
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
                    // E004: merging the trailing words into the body is one reading of
                    // a malformed `if`; a missing `elseif` keyword is another.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
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
            // E004: deleting the dangling clause discards written code.
            safety: crate::irules_checks::FixSafety::RequiresReview,
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
    pub(in crate::analyser) fn emit_w304_missing_option_terminator(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry else {
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
            // W304: `--` stops the following word being read as an option,
            // which is the whole point — a call that meant it as one changes.
            safety: crate::irules_checks::FixSafety::BehaviourHardening,
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
            // W217: as W304 — `--` re-reads the following words as variable
            // names rather than options.
            safety: crate::irules_checks::FixSafety::BehaviourHardening,
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

    /// Emit the per-item path's deferred W103 / W300 dynamic-argument
    /// diagnostics, resolving each `$var` against the **full-file**
    /// most-recent-literal-`set` scan (impossible inside an isolated body —
    /// see [`super::super::state::Analyser::pending_var_literal_checks`]).
    /// The token spans are absolute by the time the tail runs (the graft
    /// rebased them), so the result is identical to the inline whole-file
    /// emission.  No-op on the `analyse` path (the queue is only fed under
    /// `capture_global_reads`).
    pub(in crate::analyser) fn flush_var_literal_checks(&mut self) {
        let pending = std::mem::take(&mut self.pending_var_literal_checks);
        for (code, cmd_name, tok) in pending {
            match code {
                DiagCode::W103 => self.emit_w103_dynamic_first_arg(&cmd_name, tok),
                DiagCode::W300 => self.emit_w300_dynamic_path(&cmd_name, tok),
                other => unreachable!("unexpected pending var-literal code {other:?}"),
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
            // The shared per-profile registry cache: same contents as a
            // fresh build_default + load_dialect, without rebuilding the
            // whole registry per analysed document.
            let registry = tcl_registry::registry_for_dialect(self.dialect());
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
            let irules = self.dialect() == "f5-irules";
            let hits: Vec<(String, String, tcl_lexer::Span)> = self
                .result
                .stub_expr_defs
                .iter()
                .filter(|s| {
                    is_builtin_math_function(&s.name)
                        || is_builtin_expr_op(&s.name)
                        || (irules && is_irules_only_expr_op(&s.name))
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
    ///
    /// When the spec additionally marks the replacement as a drop-in
    /// rename (`deprecated_replacement_drop_in` — the replacement accepts
    /// the same argument list, e.g. `client_addr` → `IP::client_addr`),
    /// the diagnostic carries a quick fix swapping exactly the command
    /// head token for the replacement name.  Non-mechanical replacements
    /// (`ip_addr` → `IP::addr … mask …`, `use pool` → `pool`, prose like
    /// `"(removed)"`) keep the message-only warning.
    pub(in crate::analyser) fn emit_irule2002_deprecated_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect() != "f5-irules" {
            return;
        }
        let Some(spec) = self.registry.and_then(|r| r.get(cmd_name)) else {
            return;
        };
        let Some(replacement) = spec.deprecated_replacement else {
            return;
        };
        let fixes = if spec.deprecated_replacement_drop_in {
            vec![super::types::CodeFix {
                span: cmd_tok.span,
                new_text: replacement.to_string(),
                description: format!("Replace with '{replacement}'"),
                // IRULE2002: offered only when the registry marks the replacement
                // `deprecated_replacement_drop_in` — its contract is that the
                // replacement takes the deprecated command's argument list unchanged,
                // so swapping the head is a pure re-spelling. A replacement that
                // restructures arguments carries no fix at all (see the `else` arm).
                safety: crate::irules_checks::FixSafety::SemanticsEquivalent,
            }]
        } else {
            Vec::new()
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::Irule2002,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is deprecated in iRules. Use '{replacement}' instead."),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// **W143.** Warn on a direct call into a private, undocumented
    /// `::tcl::` implementation namespace (`::tcl::dict::create`,
    /// `tcl::string::totitle`, …) — real Tcl backs several built-in ensemble
    /// commands this way, and the call works, but it is never a documented
    /// or supported way to write Tcl.
    ///
    /// Registry-driven — the namespace table, its per-namespace
    /// [`TailRule`](tcl_registry::private_tcl_namespaces::TailRule), and the
    /// tail-to-subcommand resolution all live in
    /// [`tcl_registry::private_tcl_namespaces`], not here.  So a user's own
    /// namespace nested under `tcl::`, Tcl's own public `tcl::`-rooted
    /// commands (`tcl::mathop::*`, `tcl::mathfunc::*`, `tcl::prefix`), and
    /// tcllib's public packages that live *inside* a shared private
    /// namespace (`tcl::chan::memchan` and the rest of `virtchannel_base`)
    /// are never flagged.
    ///
    /// The **quick fix** is attached only when the classifier hands back a
    /// `suggestion` — i.e. when the tail is a genuine subcommand of the
    /// public ensemble in the active dialect, so `<public> <tail>` is a
    /// legal rewrite — and only for an undelimited command head (see
    /// [`Self::head_word_is_delimited`]).  A private helper with no public
    /// spelling (`::tcl::clock::GetSystemTimeZone`) still warns, but with no
    /// code-corrupting fix.
    ///
    /// Always **deferred** to [`Self::flush_w143_diagnostics`]: the two
    /// remaining suppressions are whole-file facts (a `proc` this document
    /// defines at the qualified name, and a `package require` covering it).
    pub(in crate::analyser) fn emit_w143_private_tcl_namespace(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let Some(call) = tcl_registry::private_tcl_namespaces::classify_private_tcl_namespace_call(
            cmd_name,
            registry,
            self.profile.availability_mask,
        ) else {
            return;
        };
        let message = match &call.suggestion {
            Some(suggestion) => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; use the public \
                 ensemble command instead — e.g. '{suggestion}'."
            ),
            None => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; '{}' is not a \
                 subcommand of the public '{}' ensemble, so there is no supported \
                 spelling of this call.",
                call.tail, call.public_command
            ),
        };
        let fixes = match call.suggestion {
            Some(suggestion) if !self.head_word_is_delimited(cmd_tok) => {
                vec![super::types::CodeFix {
                    span: cmd_tok.span,
                    description: format!("Replace with '{suggestion}'"),
                    new_text: suggestion,
                    // W143: the public spelling is inferred from the private one; the
                    // private implementation need not behave identically to the ensemble.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }]
            }
            _ => Vec::new(),
        };
        let diag = super::types::Diagnostic {
            code: DiagCode::W143,
            span: cmd_tok.span,
            message,
            severity: Severity::Warning,
            fixes,
        };
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        self.pending_w143
            .push((cmd_name.to_string(), ns, enforce_order, diag));
    }

    /// Whether the command head at `cmd_tok` is written as a *delimited*
    /// word — braced (`{::tcl::dict::create}`) or quoted
    /// (`"::tcl::dict::create"`).
    ///
    /// The lexer's word tokens do not span their delimiters uniformly: a
    /// braced word's span is the inner text only (the `{`/`}` are stripped),
    /// while a quoted word's span starts *on* the opening `"` and stops
    /// before the closing one.  A head-token-sized replacement therefore
    /// leaves an orphan `}` / `"` behind, and for the braced form the result
    /// (`{dict create} a 1`) is a single command word rather than the
    /// intended two — so the quick fix is suppressed for these heads and the
    /// warning stands on its own.
    fn head_word_is_delimited(&self, cmd_tok: tcl_lexer::Token) -> bool {
        cmd_tok.kind == tcl_lexer::TokenType::Str
            || self
                .source
                .as_bytes()
                .get(cmd_tok.span.start() as usize)
                .is_some_and(|b| *b == b'"')
    }

    /// Post-walk flush of the [`Self::pending_w143`] candidates.
    ///
    /// Drops a candidate when the document itself supplies the command:
    ///
    /// * **it defines it** — a `proc ::tcl::dict::mine`, a class, an
    ///   `interp alias`, a `rename` target, or an ensemble namespace at the
    ///   qualified name, resolved through the shared
    ///   [`UserResolutionFacts`] with the same namespace / load-order rules
    ///   the W002 and arity flushes use.  A `namespace eval ::tcl::dict {
    ///   proc mine … }` lands in `all_procs` fully qualified, so it is
    ///   covered by the same check;
    /// * **it requires a package that owns it** — a `package require
    ///   tcl::chan::memchan` (or of any namespace prefix of the call) means
    ///   the name belongs to that package, not to Tcl's internals.  Without
    ///   this, a file could draw W120 ("needs `package require X`") and W143
    ///   ("this is Tcl-private") for the same command, which cannot both be
    ///   true.
    ///
    /// Idempotent: drains `pending_w143`, so a second call is a no-op.
    pub(in crate::analyser) fn flush_w143_diagnostics(&mut self) {
        if self.pending_w143.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);
        let required: Vec<String> = self
            .result
            .package_requires
            .iter()
            .map(|pr| pr.name.trim_start_matches(':').to_string())
            .collect();
        let pending = std::mem::take(&mut self.pending_w143);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                diag.span.start(),
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, diag.span.start()) {
                continue;
            }
            let bare = cmd_name.trim_start_matches(':');
            if required
                .iter()
                .any(|pkg| bare == pkg || bare.starts_with(&format!("{pkg}::")))
            {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
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
        if self.dialect() != "f5-irules" || cmd_name != "matchclass" {
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
                    // IRULE2001: the rewrite restructures the argument list (it supplies
                    // the `equals` operator the two-word `matchclass` form defaults to) —
                    // exactly the case the registry's `deprecated_replacement_drop_in`
                    // contract excludes from mechanical swapping.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
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
    pub(super) fn var_name_from_token(&self, tok: tcl_lexer::Token) -> Option<String> {
        let sm = Analyser::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
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

    /// Resolve which option table an option scan should use for this call,
    /// and how its dispatch treats prefixes.
    ///
    /// `None` = abstain: the registry does not know the command, the call has
    /// no arguments, an ensemble's subcommand word is dynamic or
    /// `{*}`-expanded, the subcommand does not resolve, or the resolved table
    /// is empty.
    fn option_scan_context(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
    ) -> Option<OptionScanContext> {
        let registry = self.registry?;
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let spec = registry.get(cmd_name)?;
        // A `-prefixes 0` ensemble in this file turns prefix matching off for
        // its option table too, so abbreviations there are plain unknown
        // options rather than ambiguities.
        let prefix_matching = if self.prefixless_ensembles.contains(cmd_name) {
            tcl_registry::abbrev::PrefixMatching::Strict
        } else {
            spec.prefix_matching
        };
        let (options, parent_dialects, start_idx, sub_name) = if spec.subcommands.is_empty() {
            (spec.options, spec.dialects, 0usize, None)
        } else {
            // Ensemble-shaped: index 0 is always the subcommand word.  A
            // `{*}`-expanded or substituted word resolves to an unknown name
            // at runtime; abstain rather than guess.
            if arg_expand.first().copied().unwrap_or(false) {
                return None;
            }
            if arg_tokens
                .first()
                .is_some_and(|tok| has_substitution(&args[0], tok))
            {
                return None;
            }
            let sub =
                spec.resolve_subcommand_for_dialect(&args[0], self.profile.availability_mask)?;
            (
                sub.options,
                sub.dialects.or(spec.dialects),
                1usize,
                Some(sub.name),
            )
        };
        (!options.is_empty()).then_some(OptionScanContext {
            options,
            parent_dialects,
            start_idx,
            sub_name,
            prefix_matching,
        })
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
    /// Subcommand-scoped options resolve the first arg against the
    /// registry's own [`tcl_registry::CommandSpec::resolve_subcommand_for_dialect`]
    /// — the same unique-prefix-abbreviation resolver `string le` (⇒
    /// `length`) relies on elsewhere — rather than an exact-name match, so
    /// an abbreviated subcommand (`chan conf -inputmode` ⇒ `configure`) is
    /// still checked. An ensemble-shaped command (non-empty `subcommands`)
    /// always dispatches on its first word, so when that word can't be
    /// statically resolved (dynamic, `{*}`-expanded, unknown, or an
    /// ambiguous prefix) this abstains entirely rather than falling back to
    /// the parent `CommandSpec`'s own `options` — that table describes a
    /// different argument position and is never reachable at index 0 of a
    /// real ensemble call.
    ///
    /// Candidates are queued onto [`Analyser::pending_arity`] rather than
    /// pushed directly, so a call whose `cmd_name` resolves to a same-file
    /// user proc / class / alias / ensemble / stub — which really does
    /// dispatch to that definition, not the registry builtin the option
    /// table describes — is suppressed post-walk by
    /// [`Self::flush_arity_diagnostics`] exactly like an arity candidate.
    pub(in crate::analyser) fn emit_w004_dialect_invalid_option(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        scope_path: &[usize],
    ) {
        let Some(context) = self.option_scan_context(cmd_name, args, arg_tokens, arg_expand) else {
            return;
        };
        let OptionScanContext {
            options,
            parent_dialects,
            start_idx,
            sub_name,
            prefix_matching,
        } = context;

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
                // Profile gating (§5.2): gate.intersects(mask) plus the
                // version ceiling — an inherited option on a vendor command
                // resolves under that vendor's composed profile, and a
                // later-version option never leaks below its ceiling.
                if !self.profile.is_option_available(opt, parent_dialects) && i < arg_tokens.len() {
                    let span = arg_tokens[i].span;
                    // Message exactly: `Option 'X' on 'cmd'[ sub] is not
                    // available in the active dialect (D).`
                    let sub_suffix = sub_name.map_or(String::new(), |n| format!(" {n}"));
                    let consumed = 1 + opt.value_word_count(args, i);
                    let fixes = self.w004_remove_option_fix(arg, arg_tokens, i, consumed);
                    let diag = super::types::Diagnostic {
                        code: DiagCode::W004,
                        span,
                        message: format!(
                            "Option '{arg}' on '{cmd_name}'{sub_suffix} is not available \
in the active dialect ({}).",
                            self.dialect()
                        ),
                        severity: Severity::Warning,
                        fixes,
                    };
                    let ns = self.command_resolution_namespace(scope_path);
                    let enforce_order = !self.scope_path_in_proc_body(scope_path);
                    self.pending_arity
                        .push((cmd_name.to_string(), ns, enforce_order, diag));
                }
                i += 1 + opt.value_word_count(args, i);
                continue;
            }
            // No exact spelling or declared alias: the word may still be an
            // abbreviation. A unique prefix is legal and is left to the
            // canonical-option handling above once resolved; an ambiguous one
            // is a guaranteed runtime error (W145, issue #1234).
            let table = option_keyword_table(
                options,
                |opt| self.profile.is_option_available(opt, parent_dialects),
                prefix_matching,
            );
            match table.resolve(arg) {
                tcl_registry::abbrev::KeywordMatch::Ambiguous(candidates)
                    if i < arg_tokens.len() =>
                {
                    let candidates: Vec<String> =
                        candidates.iter().map(|s| (*s).to_string()).collect();
                    let span = arg_tokens[i].span;
                    self.emit_w145_ambiguous_option(cmd_name, sub_name, arg, &candidates, span);
                }
                tcl_registry::abbrev::KeywordMatch::Unique(canonical) => {
                    // A unique abbreviation consumes exactly what the
                    // canonical option consumes.
                    if let Some(opt) = options.iter().find(|o| o.name == canonical) {
                        i += 1 + opt.value_word_count(args, i);
                        continue;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// **W145** at an option word. Same contract as the subcommand form:
    /// name the matching candidates and offer one manual-pick fix each.
    fn emit_w145_ambiguous_option(
        &mut self,
        cmd_name: &str,
        sub_name: Option<&str>,
        word: &str,
        candidates: &[String],
        span: tcl_lexer::Span,
    ) {
        let sub_suffix = sub_name.map_or(String::new(), |n| format!(" {n}"));
        let listed = candidates
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let fixes: Vec<super::types::CodeFix> = candidates
            .iter()
            .map(|candidate| super::types::CodeFix {
                span,
                new_text: candidate.clone(),
                description: format!("Expand to '{candidate}'"),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            })
            .collect();
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W145,
            span,
            message: format!(
                "Ambiguous abbreviation '{word}' for '{cmd_name}{sub_suffix}': matches {listed}."
            ),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Build the "remove option" fix for a W004 candidate: deletes the flag
    /// token at `flag_idx` through the last of its `consumed` words (the
    /// flag plus any value word(s)).  Extends through the start of the
    /// following argument token when one exists, so the result reads
    /// cleanly with no doubled separator; otherwise ends at the last
    /// consumed token's true closing delimiter via
    /// [`tcl_lexer::word_closer_offset`] so a braced/quoted flag or value
    /// (`{-stride}`, `"-stride"`) doesn't leave a stray `}` / `"` behind —
    /// see `kcs-issue-highlight-drops-closing-delimiter.md`.
    fn w004_remove_option_fix(
        &self,
        arg: &str,
        arg_tokens: &[tcl_lexer::Token],
        flag_idx: usize,
        consumed: usize,
    ) -> Vec<super::types::CodeFix> {
        let Some(&flag_tok) = arg_tokens.get(flag_idx) else {
            return Vec::new();
        };
        let last_idx = flag_idx + consumed - 1;
        let Some(&last_tok) = arg_tokens.get(last_idx) else {
            return Vec::new();
        };
        let end = if let Some(next_tok) = arg_tokens.get(flag_idx + consumed) {
            next_tok.span.start()
        } else {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            tcl_lexer::word_closer_offset(&sm, last_tok).map_or(last_tok.span.end(), |c| c + 1)
        };
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(flag_tok.span.start(), end),
            new_text: String::new(),
            description: format!("Remove '{arg}'"),
            // W004: deleting an option the active dialect lacks drops whatever
            // the author wanted it to do.
            safety: crate::irules_checks::FixSafety::RequiresReview,
        }]
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warnings for an EXPR-role argument that uses a Tcl
    /// 9.0 string-comparison operator (`lt` / `le` / `gt` / `ge`,
    /// TIP 461) in a pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl
    /// 8.5+) in an earlier one. One diagnostic is emitted per
    /// offending operator *occurrence*, anchored at that operator's
    /// own token span rather than the whole argument, so the
    /// squiggle stays tight around the actual problem even inside a
    /// large compound expression.
    ///
    /// `content_span` is the argument's inner content: the caller has
    /// already stripped any wrapping `{}` / `""` / `$` via the
    /// token's `content_offset`, so this slices `self.source`
    /// directly instead of trusting a possibly-reconstructed text —
    /// a quoted `"$x lt $y"` argument's word text is canonicalised to
    /// `"${x} lt ${y}"` by the segmenter, which would misalign any
    /// offset computed from it.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        content_span: tcl_lexer::Span,
    ) {
        let start = content_span.start() as usize;
        let end = content_span.end() as usize;
        if start > end || end > self.source.len() {
            return;
        }
        let expr_text = &self.source[start..end];
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
        let Some((base, is_irules)) = self.w003_gates() else {
            return;
        };

        // Both the parse and the re-tokenise below force the `f5-irules`
        // dialect rather than the active one (issue #985): the lexer's
        // iRules word-operator recognition (`contains`, `and`, …) is
        // itself gated on the tokenisation dialect, so under any other
        // dialect these words would lex as plain `Word` tokens, the parse
        // below would never see a `Binary`/`Unary` application of them,
        // and it would fall back to `ExprNode::Raw` — silently hiding the
        // exact misuse this check exists to catch. `f5-irules` is a
        // strict superset of every other dialect's operator vocabulary
        // (it recognises everything `lt`/`in`/`**`/etc. does, plus the 9
        // iRules words), so forcing it here never misparses an expression
        // that would otherwise parse; the real pass/fail gate decision
        // below still keys on the active dialect's actual `(base,
        // is_irules)` facts, not this parsing dialect.
        let trimmed = expr_text.trim();
        let parsed = crate::parse_expr(trimmed, Some("f5-irules"));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }

        // Re-tokenise the same (trimmed) text at the flat lexical
        // level: every gated-keyword `Operator` token here
        // corresponds 1:1 to a `Binary` node the parse above just
        // confirmed is real (a successful parse consumes the whole
        // token stream, and `in`/`ni`/`lt`/`le`/`gt`/`ge`/the iRules
        // words are not valid prefix/atom tokens — the only way one is
        // consumed is as a genuine infix application). This gives an
        // exact per-occurrence span directly from `ExprToken::start/end`
        // without adding source-position fields to `ExprNode::Binary`
        // (which recursive/optimiser/codegen consumers across the
        // compiler pattern-match on by name, not span).
        let (tokens, _) = tcl_lexer::tokenise_expr_checked(trimmed, Some("f5-irules"));
        let gated: Vec<(&tcl_lexer::ExprToken, &'static str)> = tokens
            .iter()
            .filter(|t| t.kind == tcl_lexer::ExprTokenType::Operator)
            .filter_map(|t| gated_operator_name(&t.text, base, is_irules).map(|name| (t, name)))
            .collect();
        if gated.is_empty() {
            return;
        }

        // A mechanical rewrite is only safe to offer when the whole
        // argument IS exactly the one gated application (`2 in {1 2
        // 3}`, `$x lt $y`) with plain operands: the operator nested
        // inside a larger expression, more than one gated occurrence,
        // or an operand that isn't a single bare-word-safe atom
        // (`Binary`/`Unary`/`Ternary`/`Call` — e.g. `max($a, $b)`
        // contains an unprotected space) can't be reduced to one
        // local text replacement.
        let fix_text = (gated.len() == 1)
            .then_some(&parsed)
            .and_then(|node| match node {
                ExprNode::Binary { op, left, right }
                    if is_simple_operand(left) && is_simple_operand(right) =>
                {
                    rewrite_gated_operator(op.as_str(), &render_expr(left), &render_expr(right))
                }
                _ => None,
            });

        // Leading whitespace `expr_text.trim()` stripped, needed to
        // translate a `trimmed`-local offset back to an
        // `expr_text`-local (and then absolute source) offset.
        let trim_base = u32::try_from(expr_text.len() - expr_text.trim_start().len()).unwrap_or(0);

        for (tok, op_name) in gated {
            let op_span = tcl_lexer::Span::new(
                content_span.start() + trim_base + tok.start,
                // `ExprToken::end` is inclusive; `Span` is exclusive.
                content_span.start() + trim_base + tok.end + 1,
            );
            let mut fixes = Vec::new();
            if let Some(new_text) = fix_text.clone() {
                fixes.push(super::types::CodeFix {
                    span: tcl_lexer::Span::new(
                        content_span.start() + trim_base,
                        content_span.start()
                            + trim_base
                            + u32::try_from(trimmed.len()).unwrap_or(0),
                    ),
                    new_text,
                    description: format!(
                        "Rewrite to a form supported by dialect '{}'",
                        self.dialect()
                    ),
                    // W003: the older-dialect spelling of an operator is rarely exact —
                    // `lt` compares as strings where `<` coerces numerically.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W003,
                span: op_span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
                severity: Severity::Warning,
                fixes,
            });
        }
    }

    /// **W002** for a math function used before its introducing release —
    /// `min(…)` under 8.4, an `is*(…)` classification under 8.6.  The function
    /// dispatches to `::tcl::mathfunc::<name>`, a command that simply does not
    /// exist in the older core, so the same disabled-in-dialect diagnostic the
    /// command path emits applies to the function token in the expression.
    pub(in crate::analyser) fn emit_expr_function_dialect_diagnostics(
        &mut self,
        expr_tok: tcl_lexer::Token,
    ) {
        let Some(ceiling) = crate::tcl_expr_eval::math_func_ceiling_for_dialect(self.dialect())
        else {
            return;
        };
        for (name, span, _argc) in self.expr_function_calls(expr_tok) {
            let Some(since) = tcl_syntax::expr::mathfunc::added_in(&name) else {
                continue;
            };
            if since > ceiling {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W002,
                    span,
                    message: format!(
                        "'::tcl::mathfunc::{name}' is disabled in the active dialect profile ('{}')",
                        self.dialect()
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W003** for the multi-word `expr a b c …` form. Tcl's `expr`
    /// is the only EXPR-role command whose expression can be spread
    /// across several unbraced words (`if`/`while`/`for` require
    /// their condition to be a single Tcl word — an unbraced
    /// multi-word condition is a Tcl arity error, not valid syntax).
    /// Written this way, a gated operator keyword is necessarily its
    /// own standalone Tcl word, so its `arg_tokens` entry is already
    /// an exact, tight span — no text-offset remapping needed, unlike
    /// the single-argument path above.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_words(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        joined_text: &str,
    ) {
        if !contains_gated_word(joined_text) {
            return;
        }
        let Some((base, is_irules)) = self.w003_gates() else {
            return;
        };
        // Forced to `f5-irules` for the same reason as the braced-argument
        // path (issue #985): under any other dialect the tokeniser lexes
        // the iRules words (`contains`, `and`, …) as plain `Word`s rather
        // than operators, so the parse below would never see a valid
        // infix application and would fall back to `ExprNode::Raw`,
        // silently skipping this whole check. The real gate decision below
        // still uses the active dialect's actual `(base, is_irules)`.
        let parsed = crate::parse_expr(joined_text.trim(), Some("f5-irules"));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        for (word, tok) in args.iter().zip(arg_tokens.iter()) {
            let Some(op_name) = gated_operator_name(word, base, is_irules) else {
                continue;
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W003,
                span: tok.span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
                severity: Severity::Warning,
                // The unbraced multi-word form has no single local
                // span to rewrite in place without also re-bracing
                // the whole expression (W100's concern, not this
                // one) — left unfixed.
                fixes: Vec::new(),
            });
        }
    }

    /// The active dialect's `expr`-grammar base version and iRules
    /// identity, or `None` when the dialect string has no documented base
    /// version (nothing for W003 to check — see [`gated_operator_name`],
    /// which does the real per-operator gate comparison against this).
    fn w003_gates(&self) -> Option<(tcl_dialect::TclVersion, bool)> {
        let profile = tcl_dialect::DialectProfile::by_name(self.dialect());
        profile
            .expr_grammar_base
            .map(|base| (base, profile.is_irules()))
    }
}

/// Whether `name` is a built-in `expr` math function (`sin`, `max`, the TIP
/// 745 C99 batch, …) in *any* dialect — used by the W117 stub-shadow check,
/// which doesn't have (and doesn't need) a specific dialect to check against
/// here since a stub shadowing a function that exists in some other dialect
/// is still worth flagging. Derived from
/// [`tcl_syntax::expr::mathfunc::added_in`] — issue #983's unification —
/// rather than a hand-typed list that had already drifted once (`added_in`
/// claiming the TIP 745 batch before `dispatch()` actually implemented it).
fn is_builtin_math_function(name: &str) -> bool {
    tcl_syntax::expr::mathfunc::added_in(name).is_some()
}

/// Whether `name` is a built-in `expr` operator spelling **not** gated to
/// iRules only (`+`, `in`, `**`, `eq`, …) — used by the W117 stub-shadow
/// check. Derived from [`tcl_syntax::expr::operators`] (issue #983's
/// unification) rather than a hand-typed list: a `BinOp`/`UnaryOp` whose
/// `dialects` isn't exactly `Some(DialectSet::IRULES)` is available outside
/// iRules (`None` = ungated, `Some(TCL90_PLUS)` etc. = version-gated but not
/// dialect-*identity*-gated — both count as "built-in" here; only the nine
/// iRules word operators are excluded).
fn is_builtin_expr_op(name: &str) -> bool {
    use tcl_registry::prelude::DialectSet;
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.dialects != Some(DialectSet::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.dialects != Some(DialectSet::IRULES)
    })
}

/// Whether `name` is one of the nine iRules-only word operators (`and`,
/// `contains`, `not`, …) — see [`is_builtin_expr_op`]'s doc for the
/// derivation and why these are excluded there.
fn is_irules_only_expr_op(name: &str) -> bool {
    use tcl_registry::prelude::DialectSet;
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.dialects == Some(DialectSet::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.dialects == Some(DialectSet::IRULES)
    })
}

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
///
/// Never scans into a command's `reserved_trailing_words` (e.g.
/// `switch`'s trailing `string` + pattern-list, which C Tcl's own
/// option-scanning loop excludes structurally, regardless of shape — see
/// [`tcl_registry::spec::CommandSpec::reserved_trailing_words`]).
fn first_positional_without_terminator(
    args: &[String],
    profile: &tcl_registry::ResolvedTerminator,
) -> Option<usize> {
    let mut i = profile.scan_start;
    let n = args.len().saturating_sub(profile.reserved_trailing_words);
    while i < n {
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
pub(super) fn last_literal_set_value_for_var(
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

/// A version-gated `expr` operator: its text, whether it is word-shaped, the
/// minimum Tcl version it needs, and the TIP citation for the W003 message.
///
/// Single source for the three W003 steps — the prefilter
/// ([`contains_gated_word`]), the per-token gate check ([`gated_operator_name`]),
/// and the message ([`w003_tip_citation`]). `op`/`word_shaped`/`min_version`
/// are derived from [`tcl_syntax::expr::operators`] (issue #983's
/// unification) — only the TIP citation number is still local prose (two
/// operators can share a minimum version but not a TIP: `**` is TIP 123,
/// `in`/`ni` is TIP 201, both Tcl 8.5). Before the `tcl_syntax` derivation
/// this whole table was three separate hardcoded matches that had drifted
/// from each other (the symbolic `**` was in none of them, so
/// `expr {2 ** 3}` under tcl8.4 was a false negative).
struct GatedExprOp {
    /// The operator text as the expr lexer emits it (`in`, `**`).
    op: &'static str,
    /// Word-shaped (`in`/`lt`) operators need identifier-boundary matching in
    /// the prefilter; symbolic ones (`**`) match on any occurrence.
    word_shaped: bool,
    /// What must hold for this operator to be valid: a minimum `expr`-grammar
    /// version, or (issue #985) iRules dialect identity.
    gate: ExprOpGate,
    /// The TIP citation surfaced in the W003 message.
    tip: &'static str,
}

/// What a [`GatedExprOp`] requires of the active dialect.
#[derive(Clone, Copy)]
enum ExprOpGate {
    /// The dialect's `expr`-grammar base version must be at least this.
    MinVersion(tcl_dialect::TclVersion),
    /// The dialect must *be* iRules (identity, not a version threshold) —
    /// these 9 word operators (`contains`, `and`, …) have no
    /// `::tcl::mathop` form and exist in no other dialect at any version.
    IrulesOnly,
}

/// The TIP citation string for `spelling`'s W003 message — the one fact a
/// gated operator's `tcl_syntax::expr::operators::OperatorSpec` doesn't
/// carry (see [`GatedExprOp`]'s docs for why).
fn w003_tip_string(spelling: &str) -> &'static str {
    match spelling {
        "**" => "Tcl 8.5+ (TIP 123)",
        "in" | "ni" => "Tcl 8.5+ (TIP 201)",
        "and" | "or" | "not" | "contains" | "starts_with" | "ends_with" | "equals"
        | "matches_glob" | "matches_regex" => "the iRules/BIG-IP Tcl dialect",
        // "lt" | "le" | "gt" | "ge", and the fallback `gated_expr_ops()`
        // itself guarantees no other spelling ever reaches this function.
        _ => "Tcl 9.0+ (TIP 461)",
    }
}

/// The dialect-gated `expr` operators: version-gated (`in`/`ni`/`**` from
/// 8.5; the string comparison words `lt`/`le`/`gt`/`ge` from 9.0) — every
/// `BinOp` whose
/// [`tcl_syntax::expr::operators::OperatorSpec::expr_grammar_min_version`]
/// is `Some(_)` — plus (issue #985) the 9 iRules-only word operators, every
/// `BinOp`/`UnaryOp` whose `OperatorSpec::dialects` is exactly
/// `Some(DialectSet::IRULES)`. Computed once (not `const`: `OperatorSpec`
/// isn't cheaply iterable in a const context) and cached for the process
/// lifetime — W003 only calls into this after its own text prefilter narrows
/// to expressions that already contain a gated keyword, so this never runs
/// on a hot path.
fn gated_expr_ops() -> &'static [GatedExprOp] {
    static TABLE: std::sync::OnceLock<Vec<GatedExprOp>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        use tcl_registry::prelude::DialectSet;
        let version_gated = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                let min_version = spec.expr_grammar_min_version?;
                Some(GatedExprOp {
                    op: spec.spelling,
                    word_shaped: spec.spelling.as_bytes()[0].is_ascii_alphabetic(),
                    gate: ExprOpGate::MinVersion(min_version),
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_bin = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.dialects == Some(DialectSet::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_un = tcl_syntax::expr::operators::ALL_UNARY_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.dialects == Some(DialectSet::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        version_gated.chain(irules_bin).chain(irules_un).collect()
    })
}

/// Return `true` if `text` contains any dialect-gated expression operator (a
/// word-shaped one as a whole word, or a symbolic one such as `**` anywhere).
/// Used as a fast prefilter to skip the expression parse for expressions that
/// obviously can't trigger W003. Word-boundary matching is whitespace-aware:
/// any non-identifier byte (parentheses, operators, …) counts as a boundary,
/// matching Tcl expr's tolerance for arbitrary whitespace between tokens.
pub(super) fn contains_gated_word(text: &str) -> bool {
    let bytes = text.as_bytes();
    for g in gated_expr_ops() {
        let needle = g.op.as_bytes();
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == needle {
                let word_ok = !g.word_shaped
                    || ((i == 0 || !is_ident_continue(bytes[i - 1]))
                        && (i + n == bytes.len() || !is_ident_continue(bytes[i + n])));
                if word_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

/// The gated operator name for `word` under a dialect whose `expr`-grammar
/// base version is `base` and whose iRules-identity is `is_irules` — `None`
/// when `word` isn't one of the dialect-gated keywords, or the active
/// dialect already satisfies its gate (version threshold met, or the
/// dialect *is* iRules for an iRules-only word).
fn gated_operator_name(
    word: &str,
    base: tcl_dialect::TclVersion,
    is_irules: bool,
) -> Option<&'static str> {
    gated_expr_ops()
        .iter()
        .find(|g| {
            g.op == word
                && match g.gate {
                    ExprOpGate::MinVersion(min) => base < min,
                    ExprOpGate::IrulesOnly => !is_irules,
                }
        })
        .map(|g| g.op)
}

/// The TIP citation to surface in the W003 message for `op_name`.
fn w003_tip_citation(op_name: &str) -> &'static str {
    gated_expr_ops()
        .iter()
        .find(|g| g.op == op_name)
        .map_or("Tcl 9.0+ (TIP 461)", |g| g.tip)
}

/// Whether `node` is safe to splice as a bare Tcl word into a
/// rewritten command (`lsearch` / `string compare`) with no extra
/// quoting: it never contains an unprotected top-level space. Plain
/// atoms are always safe (`Literal` is bare digits, `String` already
/// carries its own `{}`/`""` delimiters, `Var` is `$name`/`${name}`,
/// `Command` is bracket-delimited); `Binary`/`Unary`/`Ternary` would
/// need re-parenthesising and `Call` (`max($a, $b)`) contains an
/// unprotected space after the comma, so none of those are rewritten.
fn is_simple_operand(node: &ExprNode) -> bool {
    matches!(
        node,
        ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::Var { .. }
            | ExprNode::Command { .. }
    )
}

/// Build the portable rewrite for a dialect-gated `expr` operator
/// application, or `None` if `op_name` isn't one of the six gated
/// keywords. `left`/`right` must already be safe bare-word text (see
/// [`is_simple_operand`]) — the caller is responsible for checking
/// that before rendering them.
fn rewrite_gated_operator(op_name: &str, left: &str, right: &str) -> Option<String> {
    Some(match op_name {
        "in" => format!("([lsearch -exact {right} {left}] >= 0)"),
        "ni" => format!("([lsearch -exact {right} {left}] < 0)"),
        "lt" => format!("([string compare {left} {right}] < 0)"),
        "le" => format!("([string compare {left} {right}] <= 0)"),
        "gt" => format!("([string compare {left} {right}] > 0)"),
        "ge" => format!("([string compare {left} {right}] >= 0)"),
        _ => return None,
    })
}

#[cfg(test)]
mod w117_tests {
    use super::{is_builtin_expr_op, is_builtin_math_function, is_irules_only_expr_op};

    /// The W117 stub-shadow check's own `is_builtin_*` helpers used to be
    /// three hand-typed lists (`BUILTIN_MATH_FUNCTIONS`/`BUILTIN_EXPR_OPS`/
    /// `IRULES_EXPR_OPS`) — this proves the `tcl_syntax`-derived
    /// replacements recognise exactly the same names (issue #983's
    /// unification: no behaviour change, only the source of truth moved).
    const OLD_BUILTIN_MATH_FUNCTIONS: &[&str] = &[
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
    const OLD_BUILTIN_EXPR_OPS: &[&str] = &[
        "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=",
        ">>", "^", "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "||", "~",
    ];
    const OLD_IRULES_EXPR_OPS: &[&str] = &[
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

    #[test]
    fn is_builtin_math_function_matches_the_old_hand_list_exactly() {
        for name in OLD_BUILTIN_MATH_FUNCTIONS {
            assert!(
                is_builtin_math_function(name),
                "{name}: was in the old list"
            );
        }
        // A handful of names that were never math functions.
        for name in ["puts", "expr", "+", "in", "notafunction"] {
            assert!(
                !is_builtin_math_function(name),
                "{name}: not a math function"
            );
        }
    }

    #[test]
    fn is_builtin_expr_op_matches_the_old_hand_list_exactly() {
        for name in OLD_BUILTIN_EXPR_OPS {
            assert!(is_builtin_expr_op(name), "{name}: was in the old list");
        }
        for name in OLD_IRULES_EXPR_OPS {
            assert!(
                !is_builtin_expr_op(name),
                "{name}: is iRules-only, must not count as a plain built-in"
            );
        }
        assert!(!is_builtin_expr_op("notanoperator"));
    }

    #[test]
    fn is_irules_only_expr_op_matches_the_old_hand_list_exactly() {
        for name in OLD_IRULES_EXPR_OPS {
            assert!(is_irules_only_expr_op(name), "{name}: was in the old list");
        }
        for name in OLD_BUILTIN_EXPR_OPS {
            assert!(
                !is_irules_only_expr_op(name),
                "{name}: is a plain built-in, must not count as iRules-only"
            );
        }
        assert!(!is_irules_only_expr_op("notanoperator"));
    }
}
