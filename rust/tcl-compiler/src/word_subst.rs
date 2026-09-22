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
//!    variable* is judged against a stale representation. Unlifted,
//!    `set x [llength $l]` then `puts [lindex $x 0]` then `incr x` reports
//!    nothing at all, where the same code with `lindex $x 0` on its own line
//!    reports both halves.
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

use tcl_registry::model::DocumentCommandSurface;

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
    lift(tokens, config, None)
}

/// [`lifted_calls_with_surface`] over one word rather than a whole command.
///
/// A CFG terminator keeps its value as a single [`WordExpr`] —
/// `Terminator::Return::value_word` — not as a `CommandTokens`, and the
/// commands in it run exactly as they would in a statement.
#[must_use]
pub fn lifted_calls_in_word(
    word: Option<&WordExpr>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = Vec::new();
    if let Some(word) = word {
        collect_word(word, config, Some(surface), 0, &mut out);
    }
    out
}

/// [`lifted_calls`], plus the substitutions a brace-quoted **expression** word
/// runs.
///
/// A braced word is inert to the command parser, but not to `expr`: it
/// re-parses that text as an expression and a `[…]` in it is a command the
/// statement really runs. Without a command surface there is no way to tell
/// `puts {[f]}` (literal) from `expr {[f]}` (a call), so the plain
/// [`lifted_calls`] has to take the narrow view; a caller that holds one gets
/// the whole set.
///
/// The *surface*, not the bare catalogue: a document's own
/// `# tcl-lsp: stub myexpr {value:expr}` declares an expression word exactly
/// as a shipped command does, and lowering honours that role, so a call
/// `myexpr {[id 9]}` is a call site like any other.
///
/// `return [expr {[fact [expr {$n - 1}] [expr {$n * $acc}]]}]` is the case
/// that matters. Its recursive call was invisible to the caller-evidence scan,
/// so two visible `fact 5 1` / `fact 3 1` sites read as the *complete* caller
/// set and O100 specialised `acc` to `1`: tclsh 9.0.4 prints `120` then `6`,
/// the optimised program printed `1` then `1`, and with a single call site
/// O112 deleted the base case outright and the program no longer terminated
/// (#2118).
#[must_use]
pub fn lifted_calls_with_surface(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    lift(tokens, config, Some(surface))
}

fn lift(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
) -> Vec<LiftedCall> {
    let mut out = Vec::new();
    let Some(tokens) = tokens else {
        return out;
    };
    collect_command_words(&tokens.word_exprs, config, surface, 0, &mut out);
    out
}

/// Push every substitution one command's words evaluate: the `[…]` each
/// substituting word carries, then — when the surface says so — the `[…]`
/// inside a brace-quoted word the head evaluates as an expression.
fn collect_command_words(
    words: &[WordExpr],
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    for word in words {
        collect_word(word, config, surface, depth, out);
    }
    let Some(surface) = surface else {
        return;
    };
    // No lowering pass resolves a nested substitution, so there is no
    // `interp alias` canonicalisation available here and the head's spelling
    // is the registry lookup key. A head that is itself substituted names no
    // command this walk can ask about.
    let Some(WordExpr::Literal { text: head, .. }) = words.first() else {
        return;
    };
    let args: Vec<String> = words.iter().skip(1).map(WordExpr::legacy_text).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for index in crate::ir_helpers::in_frame_expression_arg_indices(head, &arg_refs, surface) {
        // An unbraced expression word already substituted at the command
        // level, so the loop above has seen its `[…]`; descending again would
        // report the same call twice.
        let Some(WordExpr::BracedLiteral { text, source }) = words.get(index + 1) else {
            continue;
        };
        // The word's site covers the `{…}` that quoted it, so its content
        // starts one byte in. A word whose site is not direct source carries
        // no offset worth anchoring to.
        let base = (source.provenance == Provenance::Source)
            .then(|| source.span.start().saturating_add(1));
        collect_surface_text(text, base, config, surface, depth, out);
    }
}

/// Push every `[…]` an evaluated **source surface** runs.
///
/// A surface is text Tcl substitutes but which reached the IR without words of
/// its own: an expression operand, an `incr` amount, a `return` value. It is
/// not a script, so it is not segmented — the lexer's own `Cmd` tokens are
/// exactly its bracket substitutions, spans included, and each is handed to
/// the same [`push_substitution`] a word-level `[…]` takes.
///
/// `base` is the absolute offset of `text`'s first byte when the surface is a
/// verbatim source slice. Without one the recovered spans are relative to the
/// surface, so a consumer that reports positions must supply it.
#[must_use]
pub fn lifted_calls_in_text(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = Vec::new();
    collect_surface_text(text, base, config, surface, 0, &mut out);
    out
}

fn collect_surface_text(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    let Ok(tokens) = tcl_lexer::Lexer::with_config(text, config).tokenise_all() else {
        return;
    };
    let provenance = if base.is_some() {
        Provenance::Source
    } else {
        Provenance::Opaque
    };
    let base = base.unwrap_or(0);
    for token in tokens
        .iter()
        .filter(|token| token.kind == tcl_lexer::TokenType::Cmd)
    {
        let start = token.span.start() as usize;
        // A `Cmd` token's span runs to the last byte *inside* the brackets;
        // `push_substitution` parses a whole `[…]`, so the closing bracket is
        // taken back. Without one the `[` never closed, which is a parse error
        // Tcl raises rather than a command it runs.
        let end = token.span.end() as usize;
        let close = if text.as_bytes().get(end) == Some(&b']') {
            end + 1
        } else {
            end
        };
        let Some(spelling) = text.get(start..close).filter(|s| s.ends_with(']')) else {
            continue;
        };
        let site = SourceSite {
            span: Span::new(
                base.saturating_add(token.span.start()),
                base.saturating_add(u32::try_from(close).unwrap_or(u32::MAX)),
            ),
            provenance: provenance.clone(),
        };
        push_substitution(spelling, &site, config, Some(surface), depth + 1, out);
    }
}

/// Every command substitution an expression evaluates.
///
/// A fused `AssignExpr` or `ExprEval` statement, and a CFG `Branch`
/// terminator, keep a parsed expression instead of words — `set r [expr {[f]}]`
/// and `if {[f] > 5} …` reach the IR with no `CommandTokens` at all, so a
/// walk that reads only words sees neither the `f` they run nor the caller
/// evidence it carries (#2118).
///
/// `expr_base` is the absolute offset of the expression text's first byte,
/// as [`crate::ir::IfClause::condition_base`] records it; see
/// [`lifted_calls_in_text`] for what its absence costs.
#[must_use]
pub fn lifted_calls_in_expr(
    expr: &crate::expr_ast::ExprNode,
    expr_base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = Vec::new();
    collect_expr_node(expr, expr_base, config, surface, 0, &mut out);
    out
}

fn collect_expr_node(
    expr: &crate::expr_ast::ExprNode,
    expr_base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    use crate::expr_ast::ExprNode;

    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    let mut descend = |node| collect_expr_node(node, expr_base, config, surface, depth + 1, out);
    match expr {
        ExprNode::Command { text, start, .. } => {
            let site = match expr_base {
                Some(base) => SourceSite::source(Span::new(
                    base.saturating_add(*start),
                    base.saturating_add(*start)
                        .saturating_add(u32::try_from(text.len()).unwrap_or(0)),
                )),
                None => SourceSite::opaque(Span::new(0, 0)),
            };
            push_substitution(text, &site, config, Some(surface), depth, out);
        }
        ExprNode::Binary { left, right, .. } => {
            descend(left);
            descend(right);
        }
        ExprNode::Unary { operand, .. } => descend(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            descend(condition);
            descend(true_branch);
            descend(false_branch);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                descend(arg);
            }
        }
        // A word already reduced to its value: braced, its brackets are data
        // and never run; unbraced, whatever substitution it still owes really
        // is executed. The same split [`crate::ir_helpers`] makes for the
        // variable-effect walk.
        ExprNode::CompiledWord { text, braced } => {
            if !*braced {
                collect_surface_text(text, None, config, surface, depth + 1, out);
            }
        }
        // The parse gave up; its text is still evaluated, so it is read as a
        // surface rather than treated as running nothing.
        ExprNode::Raw { text } => {
            collect_surface_text(text, expr_base, config, surface, depth + 1, out);
        }
        // `"…"` substitutes inside an expression, `{…}` does not, and this
        // variant carries the source text *including* its delimiters — so the
        // delimiter is what decides. Reading every string as inert missed the
        // call in `set r [expr {"[id 9]"}]`: tclsh 8.6.18 prints `7 9` and
        // `--profile standard` folded `id`'s body to `return 7`, printing
        // `7 7` (#2118, found in review).
        ExprNode::String { text, start, .. } => {
            if let Some(inner) = quoted_operand_body(text) {
                let base = expr_base.map(|base| base.saturating_add(*start).saturating_add(1));
                collect_surface_text(inner, base, config, surface, depth + 1, out);
            }
        }
        ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
    }
}

/// The substituting body of a `"…"` expression operand, or `None` for a
/// `{…}` one.
///
/// [`ExprNode::String`] spans both spellings and keeps its delimiters, and
/// only the quoted form substitutes: tclsh 8.6.18 and 9.0.4 both print
/// `2` then `2` for `set x 1; puts [expr {"[incr x]"}]; puts $x`, and `1`
/// then `1` for the braced `{[incr x]}`.
pub(crate) fn quoted_operand_body(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    inner.contains('[').then_some(inner)
}

/// Walk one word, pushing every substitution it evaluates.
fn collect_word(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut Vec<LiftedCall>,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    match word {
        // The whole word is `[cmd …]`.
        WordExpr::CommandSubstitution { spelling, source } => {
            push_substitution(spelling, source, config, surface, depth, out);
        }
        // A compound word — `"[cmd …]"`, `a[cmd …]b`, `$v[cmd …]` — whose
        // parts evaluate left to right.
        WordExpr::Template { parts, .. } => {
            for part in parts {
                if let WordPart::CommandSubstitution { spelling, source } = part {
                    push_substitution(spelling, source, config, surface, depth, out);
                }
            }
        }
        // `{*}[cmd …]` still evaluates the substitution before expanding it.
        WordExpr::Expand { word, .. } => collect_word(word, config, surface, depth, out),
        // A braced word's `[…]` is literal to the command parser — reading
        // `word_exprs` rather than the argument text is the whole point of
        // knowing that. Whether the command it is an argument *to* re-parses
        // it as an expression is `collect_command_words`' question, not this
        // one. `Literal`, `Variable` and `Opaque` carry no substitution.
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
    surface: Option<&DocumentCommandSurface<'_>>,
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
        collect_command_words(&tokens.word_exprs, config, surface, depth + 1, out);
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

/// Recover the structured words for a value word that consists solely of one
/// command substitution.
///
/// A bare `[cmd …]` and a quoted `"[cmd …]"` have different outer syntax but
/// both evaluate the same one command substitution.  Values with literal text,
/// variables, expansion, or a braced word deliberately decline: no caller may
/// infer a command's word form from their flattened value text.
#[must_use]
pub fn whole_word_command_tokens(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
) -> Option<CommandTokens> {
    match word {
        WordExpr::CommandSubstitution { spelling, source } => {
            nested_command_words(spelling, source, config).ok()
        }
        WordExpr::Template { parts, .. } => {
            let mut substitution = None;
            for part in parts {
                match part {
                    WordPart::Text { text, .. } if text.is_empty() => {}
                    WordPart::CommandSubstitution { spelling, source }
                        if substitution.is_none() =>
                    {
                        substitution = Some((spelling, source));
                    }
                    _ => return None,
                }
            }
            let (spelling, source) = substitution?;
            nested_command_words(spelling, source, config).ok()
        }
        _ => None,
    }
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

    /// Lift with a registry, so brace-quoted expression words are descended.
    fn lift_with_registry(body: &str) -> Vec<(String, Vec<String>)> {
        let reg = registry();
        let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
        let src = format!("proc f {{x}} {{\n {body}\n}}");
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        let mut out = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let tokens = match stmt {
                    Statement::Call { tokens, .. }
                    | Statement::Barrier { tokens, .. }
                    | Statement::AssignValue { tokens, .. } => tokens.as_ref(),
                    _ => None,
                };
                out.extend(
                    lifted_calls_with_surface(tokens, config, &surface)
                        .into_iter()
                        .map(|c| (c.command, c.args)),
                );
            }
        }
        out
    }

    /// A brace-quoted word is inert to the command parser but not to `expr`,
    /// which re-parses it as an expression and runs the `[…]` in it. Which
    /// words those are is the registry's answer, so only the registry-aware
    /// lift takes them — `lifted_calls` has no way to tell `puts {[f]}` from
    /// `expr {[f]}`.
    #[test]
    fn a_braced_expression_word_runs_its_substitutions() {
        assert_eq!(
            lift_with_registry("puts [expr {$x + [incr x]}]"),
            vec![
                ("incr".to_owned(), vec!["x".to_owned()]),
                ("expr".to_owned(), vec!["{$x + [incr x]}".to_owned()]),
            ],
            "the `incr` runs, and innermost-first puts it before its `expr`"
        );
        assert!(
            lift("puts [expr {$x + [incr x]}]")
                .iter()
                .all(|(command, _)| command != "incr"),
            "without a registry the braced word stays literal"
        );
    }

    /// `expr` concatenates its words into one expression before parsing it, so
    /// a braced word anywhere in the list contributes to it. tclsh 8.6.18 and
    /// 9.0.4 both print `3` then `2` for
    /// `set x 1; puts [expr 1 + {[incr x]}]; puts $x`.
    #[test]
    fn a_concatenated_expression_word_runs_its_substitutions() {
        assert_eq!(
            lift_with_registry("puts [expr 1 + {[incr x]}]"),
            vec![
                ("incr".to_owned(), vec!["x".to_owned()]),
                (
                    "expr".to_owned(),
                    vec!["1".to_owned(), "+".to_owned(), "{[incr x]}".to_owned()],
                ),
            ]
        );
    }

    /// The descent is the registry's rule and not the brace: a braced word a
    /// command does *not* evaluate as an expression is still literal text.
    #[test]
    fn a_braced_word_that_is_not_an_expression_stays_literal() {
        assert!(
            lift_with_registry("puts {[lindex $x 0]}").is_empty(),
            "`puts` reads its word as a value, so the `[…]` never runs"
        );
        assert!(
            lift_with_registry("puts [list {[lindex $x 0]}]")
                .iter()
                .all(|(command, _)| command != "lindex"),
            "nor does `list`'s"
        );
    }

    /// A `"…"` operand substitutes inside an expression; a `{…}` one does
    /// not. Reading every [`crate::expr_ast::ExprNode::String`] as inert
    /// missed the call in `set r [expr {"[id 9]"}]`: tclsh 8.6.18 prints
    /// `7` then `9` and `--profile standard` folded `id`'s body to
    /// `return 7`, printing `7` twice.
    #[test]
    fn a_quoted_expression_operand_runs_its_substitutions() {
        let reg = registry();
        let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        for (why, source, expected) in [
            (
                "a quoted operand substitutes",
                r#""[id 9]""#,
                vec!["id".to_owned()],
            ),
            ("a braced one does not", "{[id 9]}", Vec::new()),
            (
                "and so does one welded into a larger expression",
                r#""[id 9]" eq "9""#,
                vec!["id".to_owned()],
            ),
        ] {
            let expr = tcl_syntax::expr::parser::parse_expr_for_profile(source, reg.profile());
            assert_eq!(
                lifted_calls_in_expr(&expr, None, config, &surface)
                    .iter()
                    .map(|c| c.command.clone())
                    .collect::<Vec<_>>(),
                expected,
                "{why}"
            );
        }
    }

    /// An expression surface with no words of its own — what a fused
    /// `AssignExpr` or a `Branch` terminator keeps — still runs its `[…]`.
    #[test]
    fn an_expression_surface_runs_its_substitutions() {
        let reg = registry();
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        let expr = tcl_syntax::expr::parser::parse_expr_for_profile("[id 9] > 5", reg.profile());
        let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
        let lifted = lifted_calls_in_expr(&expr, Some(100), config, &surface);
        assert_eq!(
            lifted
                .iter()
                .map(|c| (c.command.clone(), c.args.clone()))
                .collect::<Vec<_>>(),
            vec![("id".to_owned(), vec!["9".to_owned()])]
        );
        assert_eq!(
            lifted[0].span,
            tcl_lexer::Span::new(100, 106),
            "the base anchors the substitution back to its source offset"
        );
        assert_eq!(
            lifted_calls_in_text("[id 9] > 5", Some(100), config, &surface)
                .iter()
                .map(|c| c.command.clone())
                .collect::<Vec<_>>(),
            vec!["id".to_owned()],
            "the same holds for a surface read as raw text"
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
    /// Asserted on the commands and the inner call's own arguments; the outer
    /// `list`'s argument *text* keeps the raw spelling.
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

    /// The value emitter needs the exact inner word forms for a whole nested
    /// substitution, while a braced outer word must still decline.
    #[test]
    fn whole_word_command_tokens_preserve_nested_word_forms() {
        let reg = registry();
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        for (body, expected) in [
            ("puts [info exists {p\\x75b}]", "p\\x75b"),
            ("puts \"[info exists pub]\"", "pub"),
        ] {
            let src = format!("proc f {{}} {{{body}}}");
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let fu = cu.function("::f").expect("proc lowered");
            let word = fu
                .cfg
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .find_map(|stmt| match stmt {
                    Statement::Call {
                        tokens: Some(tokens),
                        ..
                    } => tokens.words().get(1),
                    _ => None,
                })
                .expect("puts argument word");
            let nested = whole_word_command_tokens(word, config).expect("nested command words");
            assert_eq!(
                nested.argv_texts,
                vec!["info".to_owned(), "exists".to_owned(), expected.to_owned()]
            );
        }

        let src = "proc f {} {puts {[info exists pub]}}";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let word = fu
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(|stmt| match stmt {
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                } => tokens.words().get(1),
                _ => None,
            })
            .expect("puts argument word");
        assert!(whole_word_command_tokens(word, config).is_none());
    }
}
