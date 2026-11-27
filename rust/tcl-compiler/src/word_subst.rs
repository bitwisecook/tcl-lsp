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
//! Builds on [`tcl_source::words`], which owns splitting one word into its
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

use crate::command_binding::SourceOriginLexical;
use tcl_lexer::Span;

use tcl_registry::model::DocumentCommandSurface;

use crate::ir::CommandTokens;
use tcl_source::words::Provenance;
use tcl_source::words::SourceSite;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

mod checked_inventory;
mod dependency_regions;
mod nested_words;
mod source_expressions;

pub use checked_inventory::checked_lifted_calls;
pub(crate) use checked_inventory::{
    OriginalAuthoredSourceSubstitutionWords, checked_argument_lifted_calls,
    checked_authored_source_substitution_words, checked_original_argument_lifted_calls,
    checked_original_lifted_calls_with_logical_source_input,
    checked_original_lifted_calls_with_metadata_context,
};
#[cfg(test)]
use checked_inventory::{OriginalLiftedCallContext, original_call_context};
pub(crate) use dependency_regions::original_source_dependency_regions;
pub use nested_words::{NestedWordsDecline, nested_command_words, whole_word_command_tokens};
pub use source_expressions::{LiftedSourceExpression, lifted_source_expressions};
pub(crate) use source_expressions::{
    lifted_representation_expressions_with_metadata_context,
    lifted_source_expressions_with_metadata_context,
    representation_expression_with_metadata_context, source_expression_from_invocation,
    source_expression_from_invocation_with_syntax_context,
};

/// Original lexical values used by single-command source emitters.
/// These retain raw escapes and grouping; they grant no command binding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct SingleCommandSourcePart {
    pub text: String,
    pub braced: bool,
    pub expand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum SingleCommandSourcePurpose {
    Script,
    CommandSubstitution,
}

/// Project one complete command through the shared configured word owner.
/// Missing geometry, malformed tails and multiple commands remain unavailable.
#[doc(hidden)]
#[must_use]
pub fn single_command_source_parts(
    text: &str,
    config: tcl_lexer::LexerConfig,
    purpose: SingleCommandSourcePurpose,
) -> Option<Vec<SingleCommandSourcePart>> {
    let image = tcl_lexer::SourceImage::document(text);
    let end = u32::try_from(text.len()).ok()?;
    let region = if purpose == SingleCommandSourcePurpose::CommandSubstitution
        && text.starts_with('[')
        && text.ends_with(']')
    {
        Span::new(1, end.checked_sub(1)?)
    } else {
        Span::new(0, end)
    };
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        image,
        region,
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() > 1 {
        return None;
    }
    let Some(command) = plan.commands.first() else {
        return Some(Vec::new());
    };
    command
        .words
        .iter()
        .map(|word| {
            let span = word.content_span().ok()?;
            let text = std::str::from_utf8(word.image().bytes().get(span.as_range())?)
                .ok()?
                .to_owned();
            Some(SingleCommandSourcePart {
                text,
                braced: word.group().kind == tcl_lexer::WordKind::Braced,
                expand: word.group().expand,
            })
        })
        .collect()
}

/// How deep a nest of `[cmd [cmd …]]` this walks before giving up. Tcl's own
/// parser caps nesting far above anything hand-written; this only has to stop
/// a generated pathological input from recursing without bound.
const MAX_SUBSTITUTION_DEPTH: u32 = 8;

/// One `[cmd …]` substitution lifted out of a word, shaped like the direct
/// invocation it behaves as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftedCall {
    /// The substitution's command word, as spelled. Dispatch identity belongs
    /// to the retained source proof in `tokens`, rather than this spelling.
    pub command: String,
    /// Retained structured command words and their exact dispatch proof, when queried.
    pub tokens: Option<CommandTokens>,
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
    /// The substitution's own words as [`nested_command_words`] recovers
    /// them, the command word at `0`; `None` when it declined.
    pub words: Option<CommandTokens>,
    /// Absolute source span of the whole `[…]`.
    pub span: Span,
}

#[derive(Default)]
struct LiftedCallCollection {
    calls: Vec<LiftedCall>,
    incomplete: bool,
    original_image: Option<tcl_lexer::SourceImage>,
}

impl LiftedCallCollection {
    fn for_original_parent(tokens: &CommandTokens, config: tcl_lexer::LexerConfig) -> Self {
        let original_image = (|| {
            let binding = tokens.source_binding.as_ref()?;
            (binding.original_argument_lexer_config(tokens)? == config).then_some(())?;
            let origin = binding.original_argument_source(tokens)?;
            origin.original_lexical_command(
                tokens.words(),
                tokens.argv.first()?.start(),
                config,
            )?;
            Some(origin.source_image().clone())
        })();
        Self {
            original_image,
            ..Self::default()
        }
    }
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
    lift(tokens, config, None).calls
}

/// A reached expression evaluator inside an original written operand.
/// This carries no dispatch or normal-completion authority for its parent.
pub(crate) struct EnteredExpressionEvaluation {
    expression: crate::expr_ast::ExprNode,
    expression_base: u32,
    span: Span,
    numbers: tcl_dialect::NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
}

impl EnteredExpressionEvaluation {
    pub(crate) fn expression(&self) -> &crate::expr_ast::ExprNode {
        &self.expression
    }

    pub(crate) fn expression_base(&self) -> u32 {
        self.expression_base
    }

    pub(crate) fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn numbers(&self) -> tcl_dialect::NumberSyntax {
        self.numbers
    }

    pub(crate) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.grammar
    }
}

/// Recover entered child evaluation independently of the enclosing handler.
/// Only the child's exact source binding supplies its implementation and syntax.
pub(crate) fn entered_expression_evaluations(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<EnteredExpressionEvaluation> {
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|call| entered_expression_evaluation(&call.tokens?, registry))
        .collect()
}

fn entered_expression_evaluation(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<EnteredExpressionEvaluation> {
    let binding = tokens.source_binding.as_ref()?;
    if binding.runtime_reachability() != crate::command_binding::SourceRuntimeReachability::Reached
    {
        return None;
    }
    let site = binding.invocation_site()?;
    let invocation =
        crate::registry_invocation::resolved_handler_invocation(registry, None, tokens)?;
    if invocation.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
        || invocation.facts.argument_offset != 0
        || tokens.words().len() != 2
    {
        return None;
    }
    let word = tokens.words().get(1)?;
    let WordExpr::BracedLiteral { text, source } = word else {
        return None;
    };
    if source.provenance != Provenance::Source {
        return None;
    }
    let head = tokens.words().first()?.source();
    if head.provenance != Provenance::Source || head.span.start() != site.offset {
        return None;
    }
    let base = crate::lowering_hooks::word_content_base(
        *tokens.argv.get(1)?,
        tokens.single_token_word.get(1).copied()?,
        text,
    )?;
    let length = u32::try_from(text.len()).ok()?;
    let end = base.checked_add(length)?;
    if site
        .source
        .source_image()
        .bytes()
        .get(base as usize..end as usize)
        != Some(text.as_bytes())
    {
        return None;
    }
    let dialect = binding.variable_context.invocation_dialect?;
    let parser = dialect.expression_parse_context(None);
    Some(EnteredExpressionEvaluation {
        expression: crate::expr_parser::parse_expr_with_syntax_context(text, &parser),
        expression_base: base,
        span: Span::new(head.span.start(), word.source().span.end()),
        numbers: dialect.numbers,
        grammar: parser.lexer_grammar,
    })
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
    let mut out = LiftedCallCollection::default();
    if let Some(word) = word {
        collect_word(word, config, Some(surface), 0, &mut out);
    }
    out.calls
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
    lift(tokens, config, Some(surface)).calls
}

/// Checked lexical substitutions in retained original source, independently
/// of whether this declaration was entered. Each child retains its own proof;
/// this inventory grants no reached operand or command dispatch authority.
pub(crate) fn original_lifted_calls_with_surface(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let Some(site) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
    else {
        return Vec::new();
    };
    if crate::registry_invocation::native_compiler_replay_source(tokens, site).is_none() {
        return Vec::new();
    }
    let config = tokens.native_lexer_config(config);
    let mut out = LiftedCallCollection::default();
    collect_command_words(&tokens.word_exprs, config, Some(surface), 0, &mut out);
    for call in &mut out.calls {
        if let Some(nested) = &mut call.tokens {
            nested.inherit_nested_bindings(tokens);
        }
    }
    out.calls
}

fn lift(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
) -> LiftedCallCollection {
    let mut out = LiftedCallCollection::default();
    let Some(tokens) = tokens else {
        return out;
    };
    if !tokens.evaluates_words() {
        return out;
    }
    let config = tokens.native_lexer_config(config);
    out = LiftedCallCollection::for_original_parent(tokens, config);
    collect_command_words(&tokens.word_exprs, config, surface, 0, &mut out);
    for lifted in &mut out.calls {
        if let Some(nested) = &mut lifted.tokens {
            nested.inherit_nested_bindings(tokens);
        }
    }
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
    out: &mut LiftedCallCollection,
) {
    for word in words {
        collect_word(word, config, surface, depth, out);
    }
    let Some(surface) = surface else {
        return;
    };
    // This lexical traversal asks only for assistance roles. Actual nested
    // dispatch proofs are attached to the lifted token carriers afterwards.
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
    let mut out = LiftedCallCollection::default();
    collect_surface_text(text, base, config, surface, 0, &mut out);
    out.calls
}

fn collect_surface_text(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    collect_surface_text_with_surface(text, base, config, Some(surface), depth, out);
}

fn collect_surface_text_with_surface(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    let Ok(tokens) = tcl_lexer::Lexer::with_config(text, config.nested()).tokenise_all() else {
        out.incomplete = true;
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
        push_substitution(spelling, &site, config, surface, depth + 1, out);
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
    let mut out = LiftedCallCollection::default();
    collect_expr_node(expr, expr_base, config, surface, 0, &mut out);
    out.calls
}

fn collect_expr_node(
    expr: &crate::expr_ast::ExprNode,
    expr_base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    use crate::expr_ast::ExprNode;

    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
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

/// The body of a `"…"` expression operand that may run a command, or `None`
/// for a `{…}` one or one with no `[`.
///
/// The quoted/braced rule is [`tcl_syntax::expr::quoted_string_body`]'s:
/// tclsh 8.6.18 and 9.0.4 both print `2` then `2` for
/// `set x 1; puts [expr {"[incr x]"}]; puts $x`, and `1` then `1` for the
/// braced `{[incr x]}`.
pub(crate) fn quoted_operand_body(text: &str) -> Option<&str> {
    tcl_syntax::expr::quoted_string_body(text).filter(|inner| inner.contains('['))
}

/// Walk one word, pushing every substitution it evaluates.
fn collect_word(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
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
                match part {
                    WordPart::CommandSubstitution { spelling, source } => {
                        push_substitution(spelling, source, config, surface, depth, out);
                    }
                    WordPart::Opaque { .. } => out.incomplete = true,
                    WordPart::Variable { spelling, source } => {
                        collect_variable_index(spelling, source, config, surface, depth, out);
                    }
                    WordPart::Text { .. } => {}
                }
            }
        }
        // `{*}[cmd …]` still evaluates the substitution before expanding it.
        WordExpr::Expand { word, .. } => collect_word(word, config, surface, depth, out),
        // A braced word's `[…]` is literal to the command parser — reading
        // `word_exprs` rather than the argument text is the whole point of
        // knowing that. Whether the command it is an argument *to* re-parses
        // it as an expression is `collect_command_words`' question, not this
        // one. Variable indices have their own configured component owner.
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {}
        WordExpr::Variable { spelling, source } => {
            collect_variable_index(spelling, source, config, surface, depth, out);
        }
        WordExpr::Opaque { .. } => out.incomplete = true,
    }
}

/// Original variable components retain their own substituted index lists.
/// Only the checked original parent supplies this arena's image and channel.
/// Lexical children retain separate source-binding and dispatch requirements.
fn collect_variable_index(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    use tcl_lexer::ExecutablePart;
    if !variable_index_may_run_command(spelling, config) {
        return;
    }
    let Ok(length) = u32::try_from(spelling.len()) else {
        out.incomplete = true;
        return;
    };
    if source.provenance != Provenance::Source || source.span.len() != length {
        out.incomplete = true;
        return;
    }
    let Some(image) = out.original_image.clone() else {
        out.incomplete = true;
        return;
    };
    if image.bytes().get(source.span.as_range()) != Some(spelling.as_bytes()) {
        out.incomplete = true;
        return;
    }
    let extent = source.span;
    let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
        image.clone(),
        extent,
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    ) else {
        out.incomplete = true;
        return;
    };
    let [root] = arena.list(arena.root()) else {
        out.incomplete = true;
        return;
    };
    if root.span != extent || !matches!(root.part, ExecutablePart::Variable { .. }) {
        out.incomplete = true;
        return;
    }
    let mut pending = vec![arena.list(arena.root()).iter()];
    while let Some(parts) = pending.last_mut() {
        let Some(part) = parts.next() else {
            pending.pop();
            continue;
        };
        match part.part {
            ExecutablePart::Variable {
                index: Some(index), ..
            } => pending.push(arena.list(index).iter()),
            ExecutablePart::Command { .. } => {
                let Some(raw) = image
                    .bytes()
                    .get(part.span.as_range())
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    out.incomplete = true;
                    return;
                };
                push_substitution(
                    raw,
                    &SourceSite::source(part.span),
                    config,
                    surface,
                    depth,
                    out,
                );
            }
            ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => {
                out.incomplete = true;
            }
            ExecutablePart::Variable { index: None, .. } | ExecutablePart::Text(_) => {}
        }
    }
}

// Braced names and escaped index brackets are separate from executable children.
fn variable_index_may_run_command(spelling: &str, config: tcl_lexer::LexerConfig) -> bool {
    if !spelling.starts_with('$') {
        return true;
    }
    match tcl_lexer::word_parts::scan_var_ref(spelling.as_bytes(), 0, config) {
        Ok(Some(reference)) if reference.next == spelling.len() => {
            reference.index.is_some_and(|index| index.contains(&b'['))
        }
        _ => true,
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
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    let Some((command, args_with_spans)) =
        crate::value_shapes::parse_command_substitution_with_spans_and_config(spelling, config)
    else {
        out.incomplete = true;
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
    out.incomplete |= nested.is_none();
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
        .map_or_default(<[WordExpr]>::to_vec);

    let mut args = Vec::with_capacity(args_with_spans.len());
    let mut arg_spans = Vec::with_capacity(args_with_spans.len());
    for (text, rel) in &args_with_spans {
        args.push(text.clone());
        arg_spans.push(Span::new(base + rel.start(), base + rel.end()));
    }
    out.calls.push(LiftedCall {
        command,
        tokens: nested.clone(),
        args,
        arg_spans,
        arg_words,
        words: nested,
        span,
    });
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod readonly_logical_substitution_tests;

#[cfg(test)]
mod native_source_lifted_inventory_owner_tests;
