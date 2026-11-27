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

//! Out-variables assigned by the command substitutions of a condition expression.

use tcl_lexer::LexerConfig;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use super::command_words::{CommandWord, tokenise_command_words};
use super::substitution_surfaces::collect_expr_commands;
use crate::expr_ast::ExprNode;
use crate::naming::normalise_var_name;
use tcl_source::limits::MAX_BRACKET_TEXT_DEPTH;

/// Out-variable names assigned by command substitutions appearing in
/// `condition` (an `if`/`while` condition expr).
///
/// A command that writes a variable named by one of its own arguments does
/// so *before* either branch runs, so a read of such a variable in the
/// guarded body is **not** read-before-set — the CFG records them as defs on
/// the synthetic `<cond>` statement so the def-use / W210 analysis sees the
/// write.  Which arguments those are is the registry's
/// [`ArgRole::VarWrite`] answer, never a name list here.
#[cfg(test)]
pub(crate) fn condition_command_out_vars(
    condition: &ExprNode,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut cmds = Vec::new();
    collect_expr_commands(condition, &mut cmds);
    let mut out = Vec::new();
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        cmd_substitution_out_vars(&tokenise_to_words(inner, registry), registry, &mut out, 0);
    }
    out
}

/// A word usable as a variable name: a *literal* identifier (so `{script}`,
/// `[sub]`, quoted words, and — via [`CommandWord::substituted`] — the
/// run-time-computed `$ref` are all rejected).
///
/// The substitution check has to come from the token stream, not the text:
/// the word's *content* spelling drops the `$`, so `set $n 1` would otherwise
/// read as a definition of a variable called `n` and silence a genuine
/// warning about it.
#[cfg(test)]
fn is_bare_var_word(word: &CommandWord) -> bool {
    !word.expanded
        && !word.substituted
        && !word.text.is_empty()
        && word
            .text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

#[cfg(test)]
pub(super) fn push_out_var(word: &CommandWord, out: &mut Vec<String>) {
    if is_bare_var_word(word) {
        let normalised = normalise_var_name(&word.text);
        if !normalised.is_empty() && !out.iter().any(|v| v == normalised) {
            out.push(normalised.to_owned());
        }
    }
}

/// Out-vars written by the command named by `words[0]`, appended to `out`.
///
/// Registry-first: the write positions come from
/// [`CommandRegistry::arg_indices_for_role`]`(…, `[`ArgRole::VarWrite`]`)`,
/// which is the same machinery the analyser's out-var handling and W210's
/// suppression harvest already consult.  It answers for every command the
/// registry knows — `catch` / `scan` / `gets` / `regexp`, plus
/// `set` / `append` / `lappend` / `incr` / `lset` / `regsub` / `lassign` /
/// `binary scan` / `dict update` / `array set` / … — so
/// `if {[set idx [lsearch $l foo]] > -1} {puts $idx}` does not look
/// read-before-set (tclsh 9.0.4 / 8.6.14 both print the index).
///
/// Bodies that run in the *caller's own* frame
/// ([`CommandRegistry::plain_body_arg_indices`] — `catch`'s script, and any
/// other `Plain` body) are descended into as well: their assignments are
/// (maybe) set once the substitution completes.  A `Structural` body
/// (`uplevel`, `namespace eval`, `proc`) writes somewhere else entirely and
/// is never harvested.
///
/// Destroyers ([`Traits::DESTROYS_VARIABLE`] — `unset`) are skipped: an
/// `unset` in a condition removes a variable rather than defining one, and
/// claiming it as a def would mask a genuine W213.
///
/// A **substituted command head** (`[$cmd length foo]`) is skipped for the
/// same reason: which command runs is run-time data, so nothing can be
/// claimed about what it writes.  The word's *content* spelling drops the
/// `$`, so without this check the head of `[$set length foo]` resolves as the
/// builtin `set` and the harvest invents a definition of `length` — enough to
/// swallow a genuine warning.  tclsh 9.0.4 / 8.6.14: `proc f {set} {if
/// {[$set length foo]} {puts $length}}; f string` errors `can't read
/// "length": no such variable`, so the read really is unsafe.
///
/// Suppress-only: over-collection avoids false warnings, under-collection
/// leaves the status quo — but a name harvested from the *wrong* command is
/// not over-collection, it is a wrong fact, so the skip is a correctness
/// requirement rather than a precision nicety.
#[cfg(test)]
fn cmd_substitution_out_vars(
    words: &[CommandWord],
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    // Native-stack safety net: `catch {catch {catch …}}` nests
    // body text inside one word, so the bracket-text cap applies. Past it,
    // stop harvesting — under-collection is the safe direction here.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let Some((cmd_word, arg_words)) = words.split_first() else {
        return;
    };
    let Some(cmd) = cmd_word.literal() else {
        return;
    };
    // Expansion changes the argv shape. This suppress-only harvest must
    // abstain rather than assign source positions to post-expansion words.
    if arg_words.iter().any(|word| word.expanded) {
        return;
    }
    let args: Vec<&str> = arg_words.iter().map(|w| w.text.as_str()).collect();
    // A command the document's release does not have writes nothing: under
    // Tcl 8.4 `lassign` raises `invalid command name`, so its words name no
    // place. Availability is the registry's answer, asked by resolving the
    // invocation under the registry's own surface — which a profile-less
    // registry leaves blind, answering for every release.
    if registry
        .resolve_invocation(cmd, &args, registry.own_surface_query())
        .is_none()
    {
        return;
    }
    if registry
        .get(cmd)
        .is_some_and(|s| s.traits.contains(Traits::DESTROYS_VARIABLE))
    {
        return;
    }
    for idx in registry.arg_indices_for_role(cmd, &args, ArgRole::VarWrite) {
        if let Some(w) = arg_words.get(idx) {
            push_out_var(w, out);
        }
    }
    for idx in registry.plain_body_arg_indices(cmd, &args) {
        // The word's *content* spelling already has the enclosing braces
        // stripped, so it is the body script text as written.  A body reached
        // through a substitution (`catch $script`) is unknown text and
        // contributes nothing.
        if let Some(body) = arg_words.get(idx).filter(|w| !w.substituted) {
            script_text_out_vars_at(&body.text, registry, out, depth);
        }
    }
}

#[cfg(test)]
fn script_text_out_vars_at(
    body: &str,
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    // The document's grammar, from the registry's profile — as
    // the standalone expression inventory does.
    for words in tokenise_command_words(body, LexerConfig::for_profile(registry.profile())) {
        cmd_substitution_out_vars(&words, registry, out, depth + 1);
    }
}

/// Tokenise source text into a flat list of words (single command).
///
/// Split under the document's grammar, taken from the dialect-selected
/// `registry`'s own profile — the same standalone command grammar,
/// so one document is tokenised one way whichever harvest asks.
#[cfg(test)]
pub(super) fn tokenise_to_words(source: &str, registry: &CommandRegistry) -> Vec<CommandWord> {
    tokenise_command_words(source, LexerConfig::for_profile(registry.profile()))
        .into_iter()
        .next()
        .unwrap_or_default()
}
