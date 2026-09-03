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
//! Four prefix shapes are recognised:
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
//! - A **namespace-wrapped prefix** (`[namespace code [list cmd extra]]`, a
//!   single `Cmd` token whose sole content is a call to a
//!   `Traits::WRAPS_COMMAND_PREFIX` command): the wrapper's own result is a
//!   command prefix, so its wrapped word is unwrapped one level and run back
//!   through the shapes above. The idiom Tk's `library/fontchooser.tcl` uses
//!   for its trace callbacks (issue #923 idx 92). A *braced* wrapped word is
//!   excluded: the analyser already walks it as a script, and a second record
//!   at the same span would double the code-lens reference count (see
//!   [`extract_wrapped_prefix_head`]).
//!
//! A dynamic head (`$var`/`[cmd]`, in any shape) can't be resolved to a
//! proc and recording it would false-fire W123, so it stays unrecorded.
//!
//! [`ArgRole::CommandPrefix`]: tcl_registry::arg_role::ArgRole::CommandPrefix

use tcl_lexer::{SourceMap, Span, Token, TokenType};
use tcl_registry::{AppendedArity, CommandRegistry, InvocationWord};
use tcl_syntax::list::find_element;

use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};

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

/// Parallel source facts for the post-head words of one invocation.
///
/// Keeping the slices together prevents call sites from accidentally pairing
/// text, tokens, expansion facts, or the source map from different commands.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandPrefixWords<'a, 'source> {
    pub(crate) texts: &'a [String],
    pub(crate) tokens: &'a [Token],
    pub(crate) single_token: &'a [bool],
    pub(crate) expanded: &'a [bool],
    pub(crate) source_map: Option<&'a SourceMap<'source>>,
}

impl<'a> CommandPrefixWords<'a, 'static> {
    /// Group post-head word facts before a source map is attached.
    pub(crate) const fn source_less(
        texts: &'a [String],
        tokens: &'a [Token],
        single_token: &'a [bool],
        expanded: &'a [bool],
    ) -> Self {
        Self {
            texts,
            tokens,
            single_token,
            expanded,
            source_map: None,
        }
    }
}

/// Extract the command-prefix callback heads of a call.
///
/// `name` is the command head; `words` describes the words after it (parallel,
/// zero-indexed), which are the same words `registry.command_prefixes` indexes.
pub(crate) fn command_prefix_invocations(
    registry: &CommandRegistry,
    name: &str,
    words: CommandPrefixWords<'_, '_>,
) -> Vec<CommandPrefixInvocation> {
    let arg_strs: Vec<&str> = words.texts.iter().map(String::as_str).collect();
    let structured: Vec<_> = words
        .texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            invocation_word(
                words.source_map,
                text,
                words.tokens.get(index).copied(),
                words.single_token.get(index).copied().unwrap_or(false),
                words.expanded.get(index).copied().unwrap_or(false),
            )
        })
        .collect();
    let mut out = Vec::new();
    for (idx, appended) in registry.command_prefixes_structured(name, &arg_strs, &structured) {
        let (Some(&tok), Some(text)) = (words.tokens.get(idx), words.texts.get(idx)) else {
            continue;
        };
        if words.expanded.get(idx).copied().unwrap_or(false) {
            continue;
        }
        if let Some((head, span, baked)) = extract_prefix_head(
            registry,
            tok,
            text,
            words.single_token.get(idx).copied().unwrap_or(false),
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
/// name; `words` describes the words after the method name, which are the
/// slices [`CommandRegistry::instance_method_command_prefixes`] indexes into.
/// Same literal-bareword guard as [`command_prefix_invocations`].
///
/// [`CommandRegistry::instance_method_command_prefixes`]: tcl_registry::CommandRegistry::instance_method_command_prefixes
pub(crate) fn instance_method_command_prefix_invocations(
    registry: &CommandRegistry,
    class: &str,
    method: &str,
    words: CommandPrefixWords<'_, '_>,
) -> Vec<CommandPrefixInvocation> {
    let arg_strs: Vec<&str> = words.texts.iter().map(String::as_str).collect();
    let structured: Vec<_> = words
        .texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            invocation_word(
                words.source_map,
                text,
                words.tokens.get(index).copied(),
                words.single_token.get(index).copied().unwrap_or(false),
                words.expanded.get(index).copied().unwrap_or(false),
            )
        })
        .collect();
    let mut out = Vec::new();
    for (idx, appended) in
        registry.instance_method_command_prefixes_structured(class, method, &arg_strs, &structured)
    {
        let (Some(&tok), Some(text)) = (words.tokens.get(idx), words.texts.get(idx)) else {
            continue;
        };
        if words.expanded.get(idx).copied().unwrap_or(false) {
            continue;
        }
        if let Some((head, span, baked)) = extract_prefix_head(
            registry,
            tok,
            text,
            words.single_token.get(idx).copied().unwrap_or(false),
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

/// Project one segmented source word into the registry's conservative word
/// vocabulary. Shared by callback-arity resolution and literal-argument
/// diagnostics so both features draw the same literal/dynamic boundary.
pub(crate) fn invocation_word<'a>(
    source_map: Option<&SourceMap<'_>>,
    text: &'a str,
    token: Option<Token>,
    single_token: bool,
    expanded: bool,
) -> InvocationWord<'a> {
    if expanded {
        return InvocationWord::Expanded;
    }
    let Some(token) = token else {
        return InvocationWord::Opaque;
    };
    if !single_token {
        return InvocationWord::Dynamic;
    }
    let delimited = source_map.is_some_and(|map| {
        matches!(
            map.source().as_bytes().get(token.span.start() as usize),
            Some(b'{' | b'"')
        )
    });
    if delimited
        && source_map.is_some_and(|map| tcl_lexer::word_closer_offset(map, token).is_none())
    {
        return InvocationWord::Opaque;
    }
    match token.kind {
        TokenType::Str => InvocationWord::Literal(text),
        TokenType::Esc if !text.contains('\\') => InvocationWord::Literal(text),
        TokenType::Var | TokenType::Cmd => InvocationWord::Dynamic,
        _ => InvocationWord::Opaque,
    }
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
        // `docs/design/contracts/lexing.md`); the
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
        return extract_list_quoted_prefix_head(registry, tok, text)
            .or_else(|| extract_wrapped_prefix_head(registry, tok, text));
    }
    None
}

/// Extract `(head, head_span, baked_arg_count)` from a command-prefix
/// argument built by a [`tcl_registry::Traits::WRAPS_COMMAND_PREFIX`] command
/// — `[namespace code [list Tracer]]`, the idiom Tk's own
/// `library/fontchooser.tcl` uses for every one of its trace callbacks.
///
/// `namespace code script` returns `::namespace inscope NS script`: invoking
/// it runs `script` in `NS` with the caller's arguments appended, so the
/// command the callback really reaches is `script`'s own head. Unwrap exactly
/// one level — the inner script must be a single call to a wrapping command,
/// whose [`ArgRole::Body`] argument is then run back through
/// [`extract_prefix_head`], so every shape that works directly
/// (`[list X a]`, a bareword, a braced `{X a}`) works wrapped too, and no
/// command name appears in this walker.
///
/// A **braced** wrapped argument is deliberately left alone.  Every recorded
/// call site must have exactly one recorder: Find All References dedupes by
/// range and would hide a duplicate, but `code_lens` counts
/// `proc_reference_spans().len()` raw, so a second record at the identical
/// span makes one callback read as *two* references.  The braced word is one
/// the analyser already walks — `Analyser::analyse_body` descends it as an
/// ordinary script and records its head — and measuring it confirms the
/// double: with this guard removed, `[namespace code {Tracer}]` reports two
/// references for one callback.  Pinned by
/// `command_table_mutation::tp_lens_counts_each_namespace_code_callback_shape_once`.
///
/// The **bareword** shape (`[namespace code Tracer]`) is *not* excluded, even
/// though `Analyser::dispatch_one_body_argument` does dispatch a bareword
/// `Body` word as a zero-argument call (the issue #923 idx 61 fix).  That
/// dispatch runs on the analyser's own statement walk, which does not descend
/// into a `[…]` command substitution — and a `namespace code` wrapper is
/// always inside one, since its result has to be substituted to be used.
/// Measured, not assumed: excluding the bareword shape here drops the
/// callback's reference count from 1 to 0 (PR #1075 review, P2 — the review's
/// double-count diagnosis holds for the braced word, which was already
/// guarded, but not for this one).
///
/// The `None` traits read below is deliberate under
/// invariant I4: this is a navigation/reference **widening** query (an
/// extra recorded reference, never a semantic specialisation), so
/// over-approximating across environments is the conservative direction.
///
/// [`ArgRole::Body`]: tcl_registry::arg_role::ArgRole::Body
fn extract_wrapped_prefix_head(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<(String, Span, usize)> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let mut segs = segment_commands_with_offset_and_config(
        inner,
        content_start,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    );
    if segs.len() != 1 {
        return None;
    }
    let seg = segs.pop()?;
    let args: Vec<&str> = seg.texts.get(1..)?.iter().map(String::as_str).collect();
    if !registry
        .invocation_traits(&seg.texts[0], &args, None)
        .contains(tcl_registry::Traits::WRAPS_COMMAND_PREFIX)
    {
        return None;
    }
    let wrapped = *registry
        .arg_indices_for_role(&seg.texts[0], &args, tcl_registry::ArgRole::Body)
        .first()?;
    let wrapped_tok = *seg.argv.get(wrapped + 1)?;
    if wrapped_tok.kind == TokenType::Str {
        return None;
    }
    extract_prefix_head(
        registry,
        wrapped_tok,
        seg.texts.get(wrapped + 1)?,
        *seg.single_token_word.get(wrapped + 1)?,
    )
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
    let seg = list_quoted_command_segment(registry, tok, text)?;
    let head_tok = seg.argv[1];
    let baked = seg.texts.len().saturating_sub(2);
    Some((seg.texts[1].clone(), head_tok.span, baked))
}

/// Extract the full segmented `list HEAD arg1 arg2 …` command from a
/// `[list …]` `Cmd` token: `Some` when the token's sole inner command is a
/// call to a [`tcl_registry::Traits::BUILDS_COMMAND_PREFIX`] command
/// (`list`) whose own first argument is a literal, resolvable bareword.
///
/// `seg.texts[1]` / `seg.argv[1]` is the resolved head; `seg.texts[2..]` /
/// `seg.argv[2..]` are the words `list` appends after it when the value is
/// later invoked as a command. Shared by [`extract_list_quoted_prefix_head`]
/// (which only needs the head/span/baked-count) and the W129 safe-interpreter
/// gate's deferred-call resolution (`analyser::commands`), which also needs
/// the full remaining words to recurse into an `apply` lambda body the same
/// way a direct `apply {…} $x` call would.
pub(crate) fn list_quoted_command_segment(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<SegmentedCommand> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let mut segs = segment_commands_with_offset_and_config(
        inner,
        content_start,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    );
    if segs.len() != 1 {
        return None;
    }
    let seg = segs.pop()?;
    list_build_is_literal(registry, &seg).then_some(seg)
}

/// Whether an already-segmented command is a call to a
/// [`tcl_registry::Traits::BUILDS_COMMAND_PREFIX`] command (`list`) whose own
/// first argument is a literal, resolvable bareword.
///
/// Split out of [`list_quoted_command_segment`] so
/// [`crate::script_arg::list_build_effective_command`] applies exactly the
/// same guard to a segment it already holds, rather than re-deriving one.
pub(crate) fn list_build_is_literal(registry: &CommandRegistry, seg: &SegmentedCommand) -> bool {
    if seg.texts.len() < 2 {
        return false;
    }
    if !registry.get(&seg.texts[0]).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
    }) {
        return false;
    }
    let Some(head_tok) = seg.argv.get(1) else {
        return false;
    };
    if head_tok.kind != TokenType::Esc
        || head_tok.in_quote
        || seg.single_token_word.get(1) != Some(&true)
    {
        return false;
    }
    seg.texts.get(1).is_some_and(|t| !looks_unresolvable(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::AppendedAritySet;

    fn trace_callback_arity(source: &str) -> AppendedArity {
        let registry = CommandRegistry::build_default();
        let segmented = crate::segmenter::segment_commands(source);
        let command = segmented
            .iter()
            .find(|command| command.name() == "trace")
            .expect("trace command is segmented");
        let source_map = SourceMap::new(source);
        let expanded = command
            .expand_word
            .as_deref()
            .and_then(|words| words.get(1..))
            .unwrap_or(&[]);
        let invocations = command_prefix_invocations(
            &registry,
            command.name(),
            CommandPrefixWords {
                texts: &command.texts[1..],
                tokens: &command.argv[1..],
                single_token: &command.single_token_word[1..],
                expanded,
                source_map: Some(&source_map),
            },
        );
        assert_eq!(invocations.len(), 1);
        invocations[0].appended
    }

    #[test]
    fn source_aware_callback_resolution_preserves_execution_arity_alternatives() {
        assert_eq!(
            trace_callback_arity("trace add execution target enter callback\n"),
            AppendedArity::Exactly(2)
        );
        assert_eq!(
            trace_callback_arity("trace add execution target leave callback\n"),
            AppendedArity::Exactly(4)
        );
        assert_eq!(
            trace_callback_arity("trace add execution target {enter leave} callback\n"),
            AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 4]))
        );
    }

    #[test]
    fn source_aware_callback_resolution_abstains_on_dynamic_and_malformed_lists() {
        assert_eq!(
            trace_callback_arity(
                "set operations {enter leave}\ntrace add execution target $operations callback\n"
            ),
            AppendedArity::Unknown
        );
        assert_eq!(
            trace_callback_arity("trace add execution target \"{enter\" callback\n"),
            AppendedArity::Unknown
        );
    }
}
