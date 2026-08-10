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

//! Resolving a **script argument that was built rather than written**.
//!
//! A Tcl script argument is usually a literal braced block — `uplevel #0 {
//! upvar #0 ::tk::Priv ::tk::Priv }` — and every pass that walks code keys on
//! that shape.  Real code just as often *builds* the script with `list`:
//!
//! ```tcl
//! uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]
//! namespace eval :: [list source [file join $::tk_library $file.tcl]]
//! ```
//!
//! Tk's own `library/tk.tcl` uses both of those.  They are not dynamic: `list`
//! packs its already-substituted arguments into exactly one command, and Tcl
//! then evaluates it deterministically.  A 3-way comparison on tclsh 9.0.4 and
//! 8.6.16 (direct `upvar`, braced `uplevel {upvar …}`, `uplevel [list upvar
//! …]`) shows the three are functionally identical — only the `[list …]` form
//! was invisible to the analyser and the semantic-token walker (issue #1138).
//!
//! [`list_quoted_script_command`] is the one predicate that answers "is this
//! `[…]` argument a statically known command, and which one?", so the
//! analyser's body gate and the LSP's declaration highlighting cannot drift
//! apart about it.
//!
//! # What it deliberately does not resolve
//!
//! The direction is *abstain on doubt*.  Only a `[…]` whose sole inner command
//! is a call to a [`Traits::BUILDS_COMMAND_PREFIX`] command (`list`, resolved
//! from the registry) with a literal, resolvable first word resolves; `[list
//! $cb …]`, `[$build x]`, `[list a; list b]`, and a widget-path head all keep
//! today's opaque-barrier behaviour.
//!
//! # Substitution timing
//!
//! Each word of the resolved command is a **pre-substituted** list element:
//! `[list set v $x]` builds `set v <the value of x>`, with `$x` read in the
//! *building* frame, before the script is evaluated.  A consumer must
//! therefore treat each word as a value that is already known to Tcl — which
//! is exactly how the segmented words read, since their spans still point at
//! the source text that produced them.  A word that is not statically knowable
//! (`::tk::Priv.$disp`) stays a dynamic word and the usual dynamic-word guards
//! skip it.
//!
//! [`Traits::BUILDS_COMMAND_PREFIX`]: tcl_registry::Traits::BUILDS_COMMAND_PREFIX

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;

use crate::segmenter::SegmentedCommand;

/// The command a `[list HEAD word …]` script argument provably *is*, or
/// `None` when the argument is not that shape.
///
/// `tok` / `text` are a command-substitution word as the segmenter reports it
/// — a [`TokenType::Cmd`] token, whose `text` keeps the surrounding `[`/`]`.
/// The returned [`SegmentedCommand`] is the *effective* command: word 0 is
/// `HEAD` and the remaining words are the ones `list` packs after it, each
/// keeping its own real source span, so a consumer's ranges land on the
/// user's text rather than on synthetic positions.
///
/// See the module docs for what is deliberately left unresolved.
#[must_use]
pub fn list_quoted_script_command(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<SegmentedCommand> {
    if tok.kind != TokenType::Cmd {
        return None;
    }
    let seg =
        crate::signature_scan::command_prefix::list_quoted_command_segment(registry, tok, text)?;
    drop_leading_word(&seg)
}

/// [`list_quoted_script_command`] for a `list HEAD word …` call that is
/// **already segmented** — the shape the semantic-token walker meets, since
/// it recurses into every `[…]` and segments its content before classifying
/// the words.
///
/// The caller owns the "is this position actually a script?" question:
/// `list` only ever returns a value, so `set data [list upvar 1 a b]` builds
/// no command at all.  Answering that needs the *enclosing* argument's role,
/// which this function cannot see.
#[must_use]
pub fn list_build_effective_command(
    registry: &CommandRegistry,
    seg: &SegmentedCommand,
) -> Option<SegmentedCommand> {
    crate::signature_scan::command_prefix::list_build_is_literal(registry, seg)
        .then(|| drop_leading_word(seg))
        .flatten()
}

/// Re-anchor `seg` one word to the right: drop the `list` head so word 0 is
/// the command the built script really invokes.
fn drop_leading_word(seg: &SegmentedCommand) -> Option<SegmentedCommand> {
    let head = *seg.argv.get(1)?;
    let start = head.span.start();
    Some(SegmentedCommand {
        span: Span::new(start, seg.span.end().max(start)),
        argv: seg.argv.get(1..)?.to_vec(),
        texts: seg.texts.get(1..)?.to_vec(),
        word_fragments: seg.word_fragments.get(1..)?.to_vec(),
        single_token_word: seg.single_token_word.get(1..)?.to_vec(),
        // Everything from the new head onwards; the dropped `list` word's own
        // tokens must not stay, or a `$`-token walk would credit them twice.
        all_tokens: seg
            .all_tokens
            .iter()
            .filter(|t| t.span.start() >= start)
            .copied()
            .collect(),
        is_partial: seg.is_partial,
        partial_delimiter: seg.partial_delimiter,
        // `{*}` markers are per-word and the leading word is gone; a list
        // build never carries one anyway (it would make the word count
        // unknowable, and `list_quoted_command_segment` requires a literal
        // head).
        expand_word: None,
        preceding_comment: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands;

    fn reg() -> &'static CommandRegistry {
        tcl_registry::registry_for_dialect("tcl8.6")
    }

    /// Segment `src` as one command and resolve its argument `idx` (1-based
    /// over the arguments) as a list-quoted script.
    fn resolve(src: &str, idx: usize) -> Option<SegmentedCommand> {
        let cmds = segment_commands(src);
        let seg = cmds.first()?;
        list_quoted_script_command(reg(), *seg.argv.get(idx)?, seg.texts.get(idx)?)
    }

    #[test]
    fn tp_resolves_a_literal_list_build_to_its_effective_command() {
        let cmd = resolve("uplevel #0 [list upvar #0 ::tk::Priv ::tk::Priv]", 2)
            .expect("a literal `[list upvar …]` resolves");
        assert_eq!(
            cmd.texts,
            vec!["upvar", "#0", "::tk::Priv", "::tk::Priv"],
            "the `list` word is dropped and the rest keep their order"
        );
        // Spans still point at the user's own text.
        let src = "uplevel #0 [list upvar #0 ::tk::Priv ::tk::Priv]";
        assert_eq!(
            &src[cmd.argv[0].span.start() as usize..cmd.argv[0].span.end() as usize],
            "upvar"
        );
    }

    #[test]
    fn tn_a_dynamic_head_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [list $cb ::tk::Priv]", 2).is_none(),
            "`[list $cb …]` names no statically known command"
        );
        assert!(
            resolve("uplevel #0 [$build ::tk::Priv]", 2).is_none(),
            "a computed substitution is not a list build"
        );
    }

    #[test]
    fn tn_a_braced_body_is_not_this_shape() {
        assert!(
            resolve("uplevel #0 {upvar #0 a b}", 2).is_none(),
            "a literal braced body is already walked as a script; this \
             predicate answers only for the built shape"
        );
    }

    #[test]
    fn tn_a_multi_command_substitution_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [list a; list b]", 2).is_none(),
            "two commands in one substitution: the value is the *last* one's \
             result, so the shape is not a single list build"
        );
    }

    #[test]
    fn tn_a_non_list_builder_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [concat upvar #0 a b]", 2).is_none(),
            "`concat` does not carry BUILDS_COMMAND_PREFIX — its result is \
             not a well-formed one-command list"
        );
    }
}
