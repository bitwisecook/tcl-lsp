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

//! Shared extraction of [`ArgRole::CommandPrefix`] callback heads.
//!
//! A command prefix (`lsort -command myCompare`, `trace add variable v w cb`,
//! `socket -server accept`) has a command name as its first word, invoked at
//! runtime with further args appended.  This module is the single place that
//! turns a call's prefix arguments into referenced command heads, so the
//! foreground analyser ([`crate::analyser`]) and the background signature scan
//! ([`super::walker`]) record them identically into `command_invocations` —
//! feeding find-references, call-hierarchy, call-graph, code-lens, W123, and
//! the callback-arity check off one substrate.
//!
//! Three prefix shapes are recognised:
//!
//! - A **literal bareword** head (a single `Esc` token, not quoted, not a
//!   `$var`/`[cmd]` substitution) — mirroring the highlight guard in
//!   `tcl_lsp_core::semantic_tokens`. Bakes 0 extra arguments.
//! - A **braced multi-word prefix** (`{cmd extra1 extra2}`, a single `Str`
//!   token) — the canonical Tcl idiom for a callback that already carries
//!   fixed leading arguments. List-parsed via [`tcl_syntax::list::find_element`]
//!   (the same grammar primitive `Tcl_SplitList` uses); the first element is
//!   the callback head and every further element is a *baked* argument the
//!   callback-arity check (`tcl_lsp_db::apply_callback_arity`) adds to the
//!   command's own appended count.
//! - A **list-quoted prefix** (`[list cmd extra1 extra2]`, a single `Cmd`
//!   token whose sole content is a call to a `Traits::BUILDS_COMMAND_PREFIX`
//!   command — `list` — with a literal bareword as its own first argument):
//!   the idiomatic way to build a callback around a dynamic value
//!   (`-command [list doSomething $x]`) rather than a fixed literal prefix.
//!   Same head/baked-count semantics as the braced shape, just constructed
//!   dynamically; the *value* of the trailing arguments isn't needed, only
//!   their count.
//!
//! A dynamic head (`$var`/`[cmd]`, in any shape) can't be resolved to a
//! proc and recording it would false-fire W123, so it stays unrecorded.
//!
//! [`ArgRole::CommandPrefix`]: tcl_registry::arg_role::ArgRole::CommandPrefix

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::{AppendedArity, CommandRegistry};
use tcl_syntax::list::find_element;

use crate::segmenter::segment_commands_with_offset;

/// A command-prefix callback head extracted from a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandPrefixInvocation {
    /// The callback command name (the prefix's first word).
    pub head: String,
    /// Source span of the head token.
    pub span: Span,
    /// How many args the calling command appends when it invokes the callback.
    pub appended: AppendedArity,
    /// Baked-in arguments already present in the prefix literal ahead of the
    /// `appended` ones — `0` for a bareword head, `N` for the further
    /// elements of a braced multi-word prefix (`{cmd a b}` bakes 2).
    pub baked: usize,
}

/// Extract the command-prefix callback heads of a call.
///
/// `name` is the command head; `arg_texts` / `arg_tokens` / `arg_single` are
/// the per-argument reconstructed text, representative token, and
/// single-token flag for the words *after* the head (parallel, 0-indexed) —
/// the same slices `registry.command_prefixes` indexes into.
pub(crate) fn command_prefix_invocations(
    registry: &CommandRegistry,
    name: &str,
    arg_texts: &[String],
    arg_tokens: &[Token],
    arg_single: &[bool],
) -> Vec<CommandPrefixInvocation> {
    let arg_strs: Vec<&str> = arg_texts.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for (idx, appended) in registry.command_prefixes(name, &arg_strs) {
        let (Some(&tok), Some(text)) = (arg_tokens.get(idx), arg_texts.get(idx)) else {
            continue;
        };
        if let Some((head, span, baked)) = extract_prefix_head(
            registry,
            tok,
            text,
            arg_single.get(idx).copied().unwrap_or(false),
        ) {
            out.push(CommandPrefixInvocation {
                head,
                span,
                appended,
                baked,
            });
        }
    }
    out
}

/// Extract the command-prefix callback heads of an object instance-method
/// dispatch `$obj method method_args…` (`$g walk … -command cb`,
/// `$t walkproc … cb`).
///
/// `class` is the receiver's resolved class; `method` is the dispatched method
/// name; the `method_*` slices are the reconstructed text, representative
/// token, and single-token flag for the words *after* the method name
/// (parallel, 0-indexed) — the slices [`CommandRegistry::instance_method_command_prefixes`]
/// indexes into.  Same literal-bareword guard as [`command_prefix_invocations`].
///
/// [`CommandRegistry::instance_method_command_prefixes`]: tcl_registry::CommandRegistry::instance_method_command_prefixes
pub(crate) fn instance_method_command_prefix_invocations(
    registry: &CommandRegistry,
    class: &str,
    method: &str,
    method_arg_texts: &[String],
    method_arg_tokens: &[Token],
    method_arg_single: &[bool],
) -> Vec<CommandPrefixInvocation> {
    let arg_strs: Vec<&str> = method_arg_texts.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for (idx, appended) in registry.instance_method_command_prefixes(class, method, &arg_strs) {
        let (Some(&tok), Some(text)) = (method_arg_tokens.get(idx), method_arg_texts.get(idx))
        else {
            continue;
        };
        if let Some((head, span, baked)) = extract_prefix_head(
            registry,
            tok,
            text,
            method_arg_single.get(idx).copied().unwrap_or(false),
        ) {
            out.push(CommandPrefixInvocation {
                head,
                span,
                appended,
                baked,
            });
        }
    }
    out
}

/// Whether `text` cannot be a resolvable command-prefix head: empty, a
/// `$var` / `[cmd]` substitution, or a Tcl/Tk widget path (`.sb`) — a
/// dynamically-bound window command, never a user proc (confirmed
/// against real Tk usage: `-yscrollcommand {.sb set}` invokes the `.sb`
/// widget's own command, which the analyser cannot resolve or
/// arity-check like an ordinary proc). Shared by both the bareword and
/// braced-list extraction paths below, so neither shape re-introduces
/// the widget-path false positive the pre-command-prefix `script()`
/// recursion had.
fn looks_unresolvable(text: &str) -> bool {
    text.is_empty()
        || text.starts_with('$')
        || text.starts_with('[')
        || crate::analyser::tk_checks::is_widget_path(text)
}

/// Extract `(head, head_span, baked_arg_count)` from a command-prefix
/// argument, or `None` when it isn't a literal, resolvable command
/// reference.
///
/// Three shapes are recognised (see the module docs): a literal bareword
/// head — a single `Esc` token, unquoted, with no leading `$`/`[`
/// substitution and not a widget path, mirroring the highlight retag guard
/// so recording and highlighting agree exactly — bakes 0; a braced
/// multi-word prefix — a single `Str` token — is list-parsed via
/// [`find_element`], with the first element as the head (subject to the
/// same guard) and every further element counted as a baked argument; a
/// list-quoted prefix — a single `Cmd` token — delegates to
/// [`extract_list_quoted_prefix_head`]. A malformed list (unmatched
/// brace/quote, typically mid-edit) or an empty list abstains rather than
/// guessing.
fn extract_prefix_head(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
    single_token: bool,
) -> Option<(String, Span, usize)> {
    if !single_token {
        return None;
    }
    if tok.kind == TokenType::Esc && !tok.in_quote && !looks_unresolvable(text) {
        return Some((text.to_string(), tok.span, 0));
    }
    if tok.kind == TokenType::Str {
        let head_el = find_element(text, 0).ok().flatten()?;
        let head = text.get(head_el.value.clone())?;
        if looks_unresolvable(head) {
            return None;
        }
        // `tok.span.start()` sits *on* the opening `{`, not the first
        // content byte (the "inner-end" word-token convention — see
        // `docs/kcs/kcs-issue-highlight-drops-closing-delimiter.md`); the
        // content itself starts `content_offset` bytes later, which is
        // where `text`'s own byte 0 (and thus `head_el`'s offsets) align.
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let head_span = Span::new(
            content_start + u32::try_from(head_el.value.start).ok()?,
            content_start + u32::try_from(head_el.value.end).ok()?,
        );
        let mut baked = 0usize;
        let mut pos = head_el.next;
        loop {
            match find_element(text, pos) {
                Ok(Some(el)) => {
                    baked += 1;
                    pos = el.next;
                }
                Ok(None) => break,
                // A malformed tail (unmatched brace/quote, typically
                // mid-edit) makes the true baked count unknowable — abstain
                // rather than silently under-counting it.
                Err(_) => return None,
            }
        }
        return Some((head.to_string(), head_span, baked));
    }
    if tok.kind == TokenType::Cmd {
        return extract_list_quoted_prefix_head(registry, tok, text);
    }
    None
}

/// Extract `(head, head_span, baked_arg_count)` from a `[list cmd a b]`
/// command-prefix argument.
///
/// `text` is the reconstructed word text for a `Cmd` token, which — unlike
/// the `Str` shape above — keeps its surrounding `[`/`]` (the segmenter's
/// convention for command-substitution words); strip them to reach the
/// inner script. That script must be exactly one call to a
/// `Traits::BUILDS_COMMAND_PREFIX` command (`list`; resolved via
/// `registry.get`, which strips a leading `::`) whose own first argument is
/// a literal, resolvable bareword — the same guard as the bareword shape,
/// applied one level in. The baked count is simply the remaining word
/// count: unlike the braced-list shape, the trailing words don't need
/// list-parsing, since `list`'s own arguments are already individually
/// segmented words, not list elements packed into one.
fn extract_list_quoted_prefix_head(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<(String, Span, usize)> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let mut segs = segment_commands_with_offset(inner, content_start);
    if segs.len() != 1 {
        return None;
    }
    let seg = segs.pop()?;
    if !registry.get(&seg.texts[0]).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
    }) {
        return None;
    }
    let head_tok = *seg.argv.get(1)?;
    if head_tok.kind != TokenType::Esc
        || head_tok.in_quote
        || seg.single_token_word.get(1) != Some(&true)
    {
        return None;
    }
    let head = seg.texts.get(1)?.as_str();
    if looks_unresolvable(head) {
        return None;
    }
    let baked = seg.texts.len().saturating_sub(2);
    Some((head.to_string(), head_tok.span, baked))
}
