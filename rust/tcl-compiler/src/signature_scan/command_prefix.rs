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
//! Only a **literal bareword** head is recorded (a single `Esc` token, not
//! quoted, not a `$var`/`[cmd]` substitution) — mirroring the highlight guard
//! in `tcl_lsp_core::semantic_tokens`.  A dynamic head can't be resolved to a
//! proc and recording it would false-fire W123; a braced multi-word prefix
//! (`{cmd extra}`) is out of scope (it fails the single-`Esc` geometry, as it
//! does for highlighting) and left as a documented follow-up.
//!
//! [`ArgRole::CommandPrefix`]: tcl_registry::arg_role::ArgRole::CommandPrefix

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::{AppendedArity, CommandRegistry};

/// A command-prefix callback head extracted from a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandPrefixInvocation {
    /// The callback command name (the prefix's first word).
    pub head: String,
    /// Source span of the head token.
    pub span: Span,
    /// How many args the calling command appends when it invokes the callback.
    pub appended: AppendedArity,
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
        if is_literal_bareword_head(tok, text, arg_single.get(idx).copied().unwrap_or(false)) {
            out.push(CommandPrefixInvocation {
                head: text.clone(),
                span: tok.span,
                appended,
            });
        }
    }
    out
}

/// Whether `tok`/`text` is a literal bareword command head — a single `Esc`
/// token, unquoted, with no leading `$`/`[` substitution.  Mirrors the
/// highlight retag guard so recording and highlighting agree exactly.
fn is_literal_bareword_head(tok: Token, text: &str, single_token: bool) -> bool {
    single_token
        && tok.kind == TokenType::Esc
        && !tok.in_quote
        && !text.is_empty()
        && !text.starts_with('$')
        && !text.starts_with('[')
}
