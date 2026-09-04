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

//! Command substitutions nested inside a statement's words.
//!
//! A `[cmd …]` written as a whole statement is an IR statement the shimmer
//! detectors walk. The same `[cmd …]` written inside another command's word
//! is not: `Statement::Call` keeps its arguments as flat text, so
//! `puts [lindex $x 0]` is one opaque call and the `lindex` inside it is
//! invisible. The runtime draws no such distinction — it evaluates the
//! substitution either way — so the gap costs twice over:
//!
//! 1. the conversion at the nested site is never reported; and
//! 2. worse, it never reaches the commit state, so *every later read of that
//!    variable* is judged against a stale representation. `set x [llength $l]`
//!    then `puts [lindex $x 0]` then `incr x` reported nothing at all, where
//!    the same code with `lindex $x 0` on its own line reports both halves.
//!
//! Builds on [`crate::word_expr`], which owns splitting one word into its
//! substitution components; this module is the statement-level view of that —
//! which *commands* a statement's words run, and in what order.
//!
//! ## Why this reads `word_exprs` and not the argument text
//!
//! Tcl substitutes `[…]` in bare and `"…"`-quoted words but **not** in braced
//! ones, and `Statement::Call::args` cannot tell them apart: `puts [lindex $x 0]`
//! and `puts {[lindex $x 0]}` both arrive as the single argument text
//! `[lindex $x 0]`. Lifting from that text would report a command Tcl never
//! runs. [`CommandTokens::word_exprs`](crate::ir::CommandTokens::word_exprs) is
//! the structured per-word syntax the segmenter already derived — it models the
//! braced word as [`WordExpr::BracedLiteral`] and the substituted one as
//! [`WordExpr::CommandSubstitution`] — so reading it is both correct and free of
//! any re-lexing.
//!
//! ## Order
//!
//! Substitutions are yielded innermost-first, then left to right — Tcl's own
//! evaluation order, so the commit state moves exactly as the runtime converts.
//! In `foo [bar [baz $x]]` the reads of `baz` land before `bar`'s.

use tcl_lexer::Span;

use crate::ir::{CommandTokens, Provenance, SourceSite, WordExpr, WordPart};

/// How deep a nest of `[cmd [cmd …]]` this walks before giving up. Tcl's own
/// parser caps nesting far above anything hand-written; this only has to stop
/// a generated pathological input from recursing without bound.
const MAX_SUBSTITUTION_DEPTH: u32 = 8;

/// One `[cmd …]` substitution lifted out of a word, shaped like the direct
/// invocation it behaves as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftedCall {
    /// The substitution's command word, as spelled. No lowering pass resolves
    /// a nested substitution, so there is no `interp alias` canonicalisation
    /// available here and the spelling is also the registry lookup key.
    pub command: String,
    /// The substitution's argument words, index-aligned with `arg_spans`.
    pub args: Vec<String>,
    /// Absolute source span of each argument word.
    pub arg_spans: Vec<Span>,
    /// The structured syntax of each argument word, index-aligned with
    /// [`Self::args`] — so a consumer can tell a braced literal from a word
    /// that substitutes without re-deciding it from the text.
    ///
    /// Empty when [`nested_command_words`] declined the substitution, or when
    /// its word count disagreed with the argument split; a consumer that reads
    /// this must keep working without it.
    pub arg_words: Vec<WordExpr>,
    /// Absolute source span of the whole `[…]`.
    pub span: Span,
}

/// Every command substitution nested in `tokens`' words, innermost-first.
///
/// Returns an empty vector for a statement whose words hold no substitution —
/// the overwhelming majority — without allocating beyond the empty `Vec`.
#[must_use]
pub fn lifted_calls(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
) -> Vec<LiftedCall> {
    let mut out = Vec::new();
    let Some(tokens) = tokens else {
        return out;
    };
    for word in &tokens.word_exprs {
        collect_word(word, config, 0, &mut out);
    }
    out
}

/// Walk one word, pushing every substitution it evaluates.
fn collect_word(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    match word {
        // The whole word is `[cmd …]`.
        WordExpr::CommandSubstitution { spelling, source } => {
            push_substitution(spelling, source, config, depth, out);
        }
        // A compound word — `"[cmd …]"`, `a[cmd …]b`, `$v[cmd …]` — whose
        // parts evaluate left to right.
        WordExpr::Template { parts, .. } => {
            for part in parts {
                if let WordPart::CommandSubstitution { spelling, source } = part {
                    push_substitution(spelling, source, config, depth, out);
                }
            }
        }
        // `{*}[cmd …]` still evaluates the substitution before expanding it.
        WordExpr::Expand { word, .. } => collect_word(word, config, depth, out),
        // `BracedLiteral` is the whole point of reading this structure rather
        // than the argument text: its `[…]` is literal, never run. `Literal`,
        // `Variable` and `Opaque` carry no substitution to lift.
        WordExpr::Literal { .. }
        | WordExpr::BracedLiteral { .. }
        | WordExpr::Variable { .. }
        | WordExpr::Opaque { .. } => {}
    }
}

/// Parse one `[…]` and push it, deepest first, together with any substitution
/// nested in its own arguments.
fn push_substitution(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    let Some((command, args_with_spans)) =
        crate::value_shapes::parse_command_substitution_with_spans_and_config(spelling, config)
    else {
        return;
    };
    let base = source.span.start();
    let leading = u32::try_from(spelling.len() - spelling.trim_start().len()).unwrap_or(0);
    let span = Span::new(
        base + leading,
        base + leading + u32::try_from(spelling.trim().len()).unwrap_or(0),
    );

    // The substitution's own words, recovered by the canonical segmenter. Its
    // structure is what decides which of them evaluate a further substitution
    // — `[list "[a]"]` and `[list b[c]d]` run one, `[list {[a]}]` does not —
    // and no test over the flat argument text can tell those apart.
    let nested = nested_command_words(spelling, source, config).ok();
    if let Some(tokens) = nested.as_ref() {
        for word in &tokens.word_exprs {
            collect_word(word, config, depth + 1, out);
        }
    }
    // Index-aligned with `args` (which start at word 1) or empty: the two
    // recoveries split words under the same `LexerConfig`, so a disagreement
    // means one of them declined the shape and the structure is not safe to
    // pair up positionally.
    let arg_words = nested
        .as_ref()
        .and_then(|tokens| tokens.word_exprs.get(1..))
        .filter(|words| words.len() == args_with_spans.len())
        .map(<[WordExpr]>::to_vec)
        .unwrap_or_default();

    let mut args = Vec::with_capacity(args_with_spans.len());
    let mut arg_spans = Vec::with_capacity(args_with_spans.len());
    for (text, rel) in &args_with_spans {
        args.push(text.clone());
        arg_spans.push(Span::new(base + rel.start(), base + rel.end()));
    }
    out.push(LiftedCall {
        command,
        args,
        arg_spans,
        arg_words,
        span,
    });
}

/// Why the words of a `[…]` command substitution could not be recovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedWordsDecline {
    /// The spelling is not one complete, non-empty `[…]` command.
    Unmodelled,
    /// It segmented, but carried no words.
    NoWords,
}

/// The structured words of a `[…]` command substitution.
///
/// The word snapshot keeps a command substitution as one opaque spelling, so
/// its inner words are recovered by running the canonical segmenter over the
/// recorded lexical extent — the same segmentation the outer command's words
/// came from, not a bespoke parser. Anything other than exactly one complete
/// command declines.
///
/// The one owner for this: the native and WASM lowerings plan a nested
/// invocation from the same words this module lifts for analysis, and a second
/// recovery that split a word differently would let the two tiers disagree
/// about what a substitution runs.
///
/// # Errors
///
/// Returns [`NestedWordsDecline`] when the spelling is not a single complete
/// `[…]` command, or when it carries no words at all.
pub fn nested_command_words(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CommandTokens, NestedWordsDecline> {
    // Word spellings arrive exact, but a substitution recovered from argument
    // text can carry the whitespace that separated it; the offset the trim
    // drops is added back so spans stay anchored where the word really sits.
    let leading = u32::try_from(spelling.len() - spelling.trim_start().len()).unwrap_or(0);
    let inner = spelling
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(NestedWordsDecline::Unmodelled)?;
    if inner.trim().is_empty() {
        return Err(NestedWordsDecline::Unmodelled);
    }
    let base = if source.provenance == Provenance::Source {
        source
            .span
            .start()
            .saturating_add(leading)
            .saturating_add(1)
    } else {
        0
    };
    let segments = crate::segmenter::segment_commands_with_offset_and_config(inner, base, config);
    let [segment] = segments.as_slice() else {
        return Err(NestedWordsDecline::Unmodelled);
    };
    if segment.is_partial {
        return Err(NestedWordsDecline::Unmodelled);
    }
    // The nested script is a sub-lex: its own text, segmented at the document
    // offset it sits at, so the word model reads `inner` and still reports
    // document spans (`SourceMap::with_base` is that sub-lexing contract).
    let sm = tcl_lexer::SourceMap::new(inner).with_base(base, 0, 0);
    let tokens = CommandTokens::from_segmented(&sm, config, segment);
    if tokens.word_exprs.is_empty() {
        return Err(NestedWordsDecline::NoWords);
    }
    Ok(tokens)
}

/// Every nested `[expr …]` in `tokens`' words, parsed, with the absolute span
/// of the substitution it came from.
///
/// `expr` concatenates its arguments, and only the single braced argument is a
/// verbatim source slice, so any other spelling is skipped rather than guessed
/// at — the same abstention the lowerer makes when it cannot anchor an
/// expression's text.
#[must_use]
pub fn lifted_exprs(
    tokens: Option<&CommandTokens>,
    profile: Option<&tcl_dialect::DialectProfile>,
) -> Vec<(crate::expr_ast::ExprNode, Span)> {
    let config = tcl_lexer::LexerConfig::for_profile(profile);
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|lifted| {
            if lifted.command != "expr" && lifted.command != "::expr" {
                return None;
            }
            let [only] = lifted.args.as_slice() else {
                return None;
            };
            let trimmed = only.trim();
            let body = trimmed
                .strip_prefix('{')
                .and_then(|t| t.strip_suffix('}'))
                .unwrap_or(trimmed);
            Some((
                tcl_syntax::expr::parser::parse_expr_for_profile(body, profile),
                lifted.span,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::ir::Statement;

    fn registry() -> tcl_registry::CommandRegistry {
        tcl_registry::CommandRegistry::build_default()
    }

    /// Every substitution lifted from the single `Call` in a one-statement proc.
    fn lift_calls(body: &str) -> Vec<LiftedCall> {
        let reg = registry();
        let src = format!("proc f {{x}} {{\n {body}\n}}");
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        let mut out = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { tokens, .. } = stmt {
                    out.extend(lifted_calls(tokens.as_ref(), config));
                }
            }
        }
        out
    }

    /// Lift the substitutions of the single `Call` in a one-statement proc.
    fn lift(body: &str) -> Vec<(String, Vec<String>)> {
        lift_calls(body)
            .into_iter()
            .map(|c| (c.command, c.args))
            .collect()
    }

    /// The whole point of reading `word_exprs` rather than the argument text:
    /// Tcl substitutes `[…]` in a bare or quoted word and **not** in a braced
    /// one, but `Statement::Call::args` renders all three identically as
    /// `[lindex $x 0]`. Lifting from that text would report a command Tcl
    /// never runs.
    #[test]
    fn braced_word_is_not_a_substitution() {
        assert_eq!(
            lift("puts [lindex $x 0]"),
            vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])],
            "a bare `[…]` word runs"
        );
        assert_eq!(
            lift("puts \"[lindex $x 0]\""),
            vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])],
            "a quoted `[…]` runs too"
        );
        assert!(
            lift("puts {[lindex $x 0]}").is_empty(),
            "a braced `[…]` is literal text, never run"
        );
    }

    /// Substitutions come back innermost-first — Tcl's evaluation order, so a
    /// consumer replaying them moves its state exactly as the runtime does.
    #[test]
    fn nested_substitutions_are_innermost_first() {
        assert_eq!(
            lift("puts [list [lindex $x 0]]"),
            vec![
                ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
                ("list".to_owned(), vec!["[lindex $x 0]".to_owned()]),
            ]
        );
    }

    /// A substitution embedded in a larger word is still evaluated.
    #[test]
    fn substitution_inside_a_compound_word_is_lifted() {
        assert_eq!(
            lift("puts a[lindex $x 0]b"),
            vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])]
        );
    }

    /// A word with nothing to run costs nothing.
    #[test]
    fn words_without_substitutions_lift_nothing() {
        assert!(lift("puts $x").is_empty());
        assert!(lift("puts plain").is_empty());
    }

    /// A substitution nested in a *quoted* word of another substitution still
    /// runs — Tcl substitutes inside `"…"`. Recovering the nested command's
    /// own words is what sees it; the argument text `"[lindex $x 0]"` starts
    /// with a quote, so no `[`-prefix test over that text ever could.
    ///
    /// Asserted on the commands and the inner call's own arguments: the outer
    /// `list`'s argument *text* is the compatibility spelling, which this
    /// change does not touch.
    #[test]
    fn substitution_inside_a_quoted_nested_word_is_lifted() {
        let calls = lift_calls("puts [list \"[lindex $x 0]\"]");
        assert_eq!(
            calls.iter().map(|c| c.command.as_str()).collect::<Vec<_>>(),
            vec!["lindex", "list"]
        );
        assert_eq!(calls[0].args, vec!["$x".to_owned(), "0".to_owned()]);
    }

    /// The same for a substitution welded into a larger nested word.
    #[test]
    fn substitution_welded_into_a_nested_word_is_lifted() {
        assert_eq!(
            lift("puts [list a[lindex $x 0]b]"),
            vec![
                ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
                ("list".to_owned(), vec!["a[lindex $x 0]b".to_owned()]),
            ]
        );
    }

    /// …and the brace rule holds one level down too: a braced argument of a
    /// nested command is literal text, not a command to run.
    #[test]
    fn braced_word_of_a_nested_substitution_is_not_run() {
        assert_eq!(
            lift("puts [list {[lindex $x 0]}]"),
            vec![("list".to_owned(), vec!["{[lindex $x 0]}".to_owned()])]
        );
    }

    /// A substitution in the *command* position runs before the command it
    /// spells is looked up.
    #[test]
    fn substitution_in_the_command_word_is_lifted() {
        assert_eq!(
            lift("puts [[lindex $x 0] 1]"),
            vec![
                ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
                ("[lindex $x 0]".to_owned(), vec!["1".to_owned()]),
            ]
        );
    }

    /// `arg_words` is the segmenter's structure for the same words `args`
    /// spells, index-aligned, so a consumer can tell a braced literal from a
    /// word that substitutes without re-deciding it from the text.
    #[test]
    fn arg_words_align_with_args() {
        let calls = lift_calls("puts [list {a b} $x [set y] plain]");
        // `[set y]` is evaluated first, so the outer `list` is lifted last.
        let [_, lifted] = calls.as_slice() else {
            panic!("expected the inner `set` then the outer `list`, got {calls:?}");
        };
        assert_eq!(lifted.command, "list");
        assert_eq!(lifted.arg_words.len(), lifted.args.len());
        assert!(matches!(
            lifted.arg_words.as_slice(),
            [
                WordExpr::BracedLiteral { .. },
                WordExpr::Variable { .. },
                WordExpr::CommandSubstitution { .. },
                WordExpr::Literal { .. },
            ]
        ));
    }
}
