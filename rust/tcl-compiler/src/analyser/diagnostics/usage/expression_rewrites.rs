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

//! A redundant nested `expr` (W114) and `==` / `!=` on strings (W110), with their rewrites.

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use crate::optimiser::helpers::expr_simplify::eq_ne_compares_as_strings;
use tcl_core_types::DiagCode;

impl Analyser {
    /// W114: actual selected nested expression evaluation in an original
    /// expression operand. Whole-word and child-region owners retain source
    /// configuration; each nested command independently selects its descriptor.
    pub(in crate::analyser) fn emit_w114_original_nested_expr(
        &mut self,
        outer: &crate::analyser::diagnostic_registry::OriginalDiagnosticSource,
        argument: usize,
    ) {
        let Some(word) = outer.word(argument) else {
            return;
        };
        if outer.written_index(argument).is_none()
            || word.image().bytes() != self.source.as_bytes()
            || !self
                .result
                .matches_original_source_image(word.image(), self.file_lexer_config())
        {
            return;
        }
        let Some(regions) = self.w114_original_regions(word) else {
            return;
        };
        for (substitution, body) in regions {
            let Some(text) = self.source.get(body.as_range()) else {
                continue;
            };
            let commands = crate::segmenter::segment_commands_with_offset_and_config(
                text,
                body.start(),
                tcl_lexer::SourceSegmentationPurpose::NestedScript.config(word.config()),
            );
            let [command] = commands.as_slice() else {
                continue;
            };
            if command.is_partial {
                continue;
            }
            let Some(inner) = self.original_diagnostic_source_for_segment(command) else {
                continue;
            };
            let Some(selected) = inner.registry() else {
                continue;
            };
            if selected.with_schema(|schema| schema.semantics.successful_handler)
                != Some(Some(
                    tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments,
                ))
            {
                continue;
            }
            let fixes = Self::w114_original_unwrap_fix(word, selected, substitution)
                .into_iter()
                .collect();
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W114,
                    substitution,
                    "Redundant nested [expr] \u{2014} already in expression context".to_string(),
                    Severity::Warning,
                )
                .with_fixes(fixes),
            );
            return;
        }
    }

    /// Compatibility for the existing whole-word currency fixture. The actual
    /// retained parent command reissues the same selected production owner.
    #[cfg(test)]
    pub(super) fn emit_w114_redundant_nested_expr(&mut self, word: &tcl_lexer::NativeWord) {
        let config = self.file_lexer_config();
        if word.image().bytes() != self.source.as_bytes()
            || word.config() != config
            || !self
                .result
                .matches_original_source_image(word.image(), config)
        {
            return;
        }
        let Some((retained, offset)) = self
            .head_identities
            .original_written_word_at_span_in_source(word.image(), word.span(), config)
        else {
            return;
        };
        if retained != *word {
            return;
        }
        let Some((command, _)) = self
            .head_identities
            .invocation_at_source("", offset)
            .original_recorded_command()
        else {
            return;
        };
        let Some(original) = self.original_diagnostic_source_for_segment(&command) else {
            return;
        };
        let matches = (0..original.arguments().len())
            .filter(|&argument| {
                original.written_index(argument).is_some() && original.word(argument) == Some(word)
            })
            .collect::<Vec<_>>();
        let [argument] = matches.as_slice() else {
            return;
        };
        self.emit_w114_original_nested_expr(&original, *argument);
    }

    fn w114_original_regions(
        &self,
        word: &tcl_lexer::NativeWord,
    ) -> Option<Vec<(tcl_lexer::Span, tcl_lexer::Span)>> {
        if word.group().kind != tcl_lexer::WordKind::Braced {
            // This operand is substituted before the outer expression receives
            // it. Its actual lexical command regions support advice, not unwrap.
            let mut regions = Vec::new();
            for part in word.executable_parts().all_parts() {
                match part.part {
                    tcl_lexer::ExecutablePart::ParseError(_) => return None,
                    tcl_lexer::ExecutablePart::Command { body } => regions.push((part.span, body)),
                    _ => {}
                }
            }
            return Some(regions);
        }
        let content = word.content_span().ok()?;
        let text = std::str::from_utf8(word.image().bytes().get(content.as_range())?).ok()?;
        let parser_context = self.original_expression_parser_context()?;
        let original = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            text,
            &parser_context,
            word.config(),
        )?;
        // A command embedded in a quoted operand or an index only supplies
        // text to that operand (`"[expr {$p * 100}]%"`), so it is not
        // redundant expression context.
        original
            .command_regions()?
            .into_iter()
            .filter(|region| region.whole_operand)
            .map(|region| {
                let shift = |span: tcl_lexer::Span| {
                    Some(tcl_lexer::Span::new(
                        content.start().checked_add(span.start())?,
                        content.start().checked_add(span.end())?,
                    ))
                };
                Some((shift(region.substitution)?, shift(region.body)?))
            })
            .collect()
    }

    fn w114_original_unwrap_fix(
        outer: &tcl_lexer::NativeWord,
        inner: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        span: tcl_lexer::Span,
    ) -> Option<super::super::types::CodeFix> {
        if outer.group().kind != tcl_lexer::WordKind::Braced
            || contains_string_comparison_word(outer.try_text().ok()?)
            || inner.words().arguments().len() != 1
            || inner.written_index(0)? != 0
        {
            return None;
        }
        let argument = inner.word(0)?;
        if argument.group().kind != tcl_lexer::WordKind::Braced || argument.image() != outer.image()
        {
            return None;
        }
        let content = argument.content_span().ok()?;
        let expression = std::str::from_utf8(argument.image().bytes().get(content.as_range())?)
            .ok()?
            .trim();
        if expression.is_empty() {
            return None;
        }
        let new_text = if is_expr_atom(expression, argument.config()) {
            expression.to_owned()
        } else {
            format!("({expression})")
        };
        Some(super::super::types::CodeFix {
            span,
            new_text,
            description: "Unwrap the nested `expr`".to_owned(),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
    }

    /// **W110.** Emit "use `eq`/`ne` instead of `==`/`!=` for
    /// string comparison" hints on the EXPR-role argument of
    /// commands like `if`, `while`, `for`, `expr`.
    ///
    /// Fires only where the rewrite is proven to keep the result: in a
    /// braced argument, a `==` / `!=` one of whose operands is a fixed string
    /// that is not a number in any release, so Tcl already compares the two as
    /// strings ([`eq_ne_compares_as_strings`]). `$x == "42"` is a numeric
    /// compare when `x` is `42.0`, and `"$x" == 1` when `x` is `1.0`, so
    /// neither draws the hint; `$x == "foo"` does.
    ///
    /// `expr_text` is the post-substitution body of the EXPR-role
    /// argument (already brace-stripped) — the caller is
    /// responsible for joining multi-arg `expr` invocations with
    /// spaces before calling.  `diag_span` is the fallback span (the
    /// source range of the argument token, or the full token range for
    /// `expr`); the diagnostic anchors on the offending *operator*
    /// itself when `anchor` lets its offset map back to real source
    /// bytes.  The code fix always spans the whole argument
    /// (`diag_span`) — the rewrite replaces the full expression text.
    pub(in crate::analyser) fn emit_w110_string_eq_ne(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
        anchor: &W110Anchor<'_>,
    ) {
        // Quick bail-out: no equality operator at all.
        if !expr_text.contains("==") && !expr_text.contains("!=") {
            return;
        }
        let Some(parser_context) = self.original_expression_parser_context() else {
            return;
        };
        let parsed =
            tcl_syntax::expr::parser::parse_expr_with_syntax_context(expr_text, &parser_context);
        // ``ExprNode::Raw`` means the expression was unparseable.
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        // Only a braced argument reaches `expr` as written; any other word is
        // substituted first, so its operands are not the ones parsed here.
        let braced = matches!(anchor, W110Anchor::ArgToken(tok)
            if self.source.as_bytes().get(tok.span.start() as usize) == Some(&b'{'));
        if !braced {
            return;
        }
        let W110Anchor::ArgToken(token) = anchor else {
            return;
        };
        let Some(word) = self.retained_value_word(*token) else {
            return;
        };
        let Ok(content) = word.content_span() else {
            return;
        };
        if word.group().kind != tcl_lexer::WordKind::Braced
            || word.span() != diag_span
            || word.image().bytes().get(content.as_range()) != Some(expr_text.as_bytes())
        {
            return;
        }
        let config = word.config();
        let Some(original_expression) =
            tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
                expr_text,
                &parser_context,
                config,
            )
        else {
            return;
        };
        if !original_expression.matches_tree(&parsed) {
            return;
        }
        let matched_ops = find_original_string_eq_ne_ops(&parsed, &original_expression);
        if matched_ops.is_empty() {
            return;
        }
        let (first_op, first_off) = matched_ops[0];
        let (op_text, replacement) = match first_op {
            BinOp::Eq => ("==", "eq"),
            BinOp::Ne => ("!=", "ne"),
            _ => unreachable!("find_string_eq_ne_ops only returns Eq/Ne"),
        };
        // Parser positions are relative to the verbatim braced contents.
        let span = first_off
            .and_then(|off| self.w110_operator_span(anchor, off as usize, op_text))
            .unwrap_or(diag_span);
        // The fix rewrites exactly the proven operators, in place, and only
        // when every one of them maps back to its source bytes.
        let mut fixes = Vec::new();
        if let Some(fix) = self.w110_operator_fix(anchor, &matched_ops, replacement) {
            fixes.push(fix);
        }
        let message = format!(
            "Use '{replacement}' instead of '{op_text}' for string \
comparison in expressions to avoid ambiguous \
numeric/string coercion."
        );
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::W110, span, message, Severity::Hint)
                .with_fixes(fixes),
        );
    }

    /// One edit over the proven operators: each `==` / `!=` becomes `eq` /
    /// `ne`, spaced from its neighbours, and every other byte stays as
    /// written. Each rewritten operator compares as strings already
    /// ([`eq_ne_compares_as_strings`]), so the rewrite keeps the result.
    fn w110_operator_fix(
        &self,
        anchor: &W110Anchor<'_>,
        ops: &[(BinOp, Option<u32>)],
        replacement: &str,
    ) -> Option<super::super::types::CodeFix> {
        let mut spans = Vec::with_capacity(ops.len());
        for (op, off) in ops {
            let (from, to) = match op {
                BinOp::Eq => ("==", "eq"),
                BinOp::Ne => ("!=", "ne"),
                _ => return None,
            };
            let span = self.w110_operator_span(anchor, (*off)? as usize, from)?;
            spans.push((span.start() as usize, span.end() as usize, to));
        }
        spans.sort_unstable();
        let (first, last) = (spans.first()?.0, spans.last()?.1);
        let bytes = self.source.as_bytes();
        let spaced = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_whitespace);
        let mut new_text = String::new();
        let mut at = first;
        for (start, end, to) in spans {
            new_text.push_str(self.source.get(at..start)?);
            if start > 0 && !spaced(start - 1) {
                new_text.push(' ');
            }
            new_text.push_str(to);
            if !spaced(end) {
                new_text.push(' ');
            }
            at = end;
        }
        Some(super::super::types::CodeFix {
            span: tcl_lexer::Span::new(u32::try_from(first).ok()?, u32::try_from(last).ok()?),
            new_text,
            description: format!("Use '{replacement}' for string comparison"),
            safety: crate::irules_checks::FixSafety::SemanticsEquivalent,
        })
    }

    /// Map a W110 operator offset (within the emitter's `expr_text`) to
    /// its absolute source span, verifying the source bytes actually spell
    /// `op_text` there (reconstructed argument texts fail the check →
    /// `None`, and the caller falls back to the argument span).
    fn w110_operator_span(
        &self,
        anchor: &W110Anchor<'_>,
        off: usize,
        op_text: &str,
    ) -> Option<tcl_lexer::Span> {
        let candidate = match anchor {
            // Single argument word: content starts `content_offset` bytes
            // into the token span.
            W110Anchor::ArgToken(tok) => {
                tok.span.start() as usize + usize::from(tok.content_offset) + off
            }
            // `expr $a == "x"` multi-word form: `expr_text` is
            // `args.join(" ")`, so locate which word the offset falls in
            // and anchor at that word's own token.
            W110Anchor::JoinedWords { args, tokens } => {
                let mut cum = 0usize;
                let mut found = None;
                for (i, arg) in args.iter().enumerate() {
                    if off < cum + arg.len() {
                        found = Some((i, off - cum));
                        break;
                    }
                    cum += arg.len() + 1;
                }
                let (word_idx, in_word) = found?;
                let tok = tokens.get(word_idx)?;
                tok.span.start() as usize + usize::from(tok.content_offset) + in_word
            }
        };
        // Byte-verify: the span is only trustworthy when the source spells
        // the operator at the candidate position.
        if self.source.get(candidate..candidate + op_text.len())? != op_text {
            return None;
        }
        Some(tcl_lexer::Span::new(
            u32::try_from(candidate).ok()?,
            u32::try_from(candidate + op_text.len()).ok()?,
        ))
    }
}

/// How [`Analyser::emit_w110_string_eq_ne`] maps an operator offset within
/// its `expr_text` back to source bytes.
pub(in crate::analyser) enum W110Anchor<'a> {
    /// The single EXPR-role argument's own token — `expr_text` is that
    /// word's (possibly brace-stripped) content.
    ArgToken(tcl_lexer::Token),
    /// The joined multi-word `expr $a == "x"` form — `expr_text` is
    /// `args.join(" ")` over these words/tokens.
    JoinedWords {
        /// The argument word texts that were joined.
        args: &'a [String],
        /// The words' representative tokens, same order.
        tokens: &'a [tcl_lexer::Token],
    },
}

/// Find the first `[`-`expr`-whitespace sequence in `slice` (the
/// bracket-expression spelling) and return
/// `(open_bracket_index, matching_close_bracket_index)`.  The close is
/// located by a depth scan; an unmatched `[` falls back to the last
/// byte.  Returns `None` when no nested `[expr ` is present.
#[cfg(test)]
pub(in super::super) fn first_nested_expr(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let len = bytes.len();
    // Bracket-substitution nesting depth.  Only a *top-level* (depth-0)
    // `[expr …]` command substitution is "already in expression context" and
    // therefore redundant.  An `[expr …]` nested inside another command
    // substitution's arguments — e.g. `if {[myCmd [expr {1+1}]]}` — is a fresh
    // command-argument context, not an expression context, so it must NOT be
    // flagged.
    let mut depth: i32 = 0;
    let mut open = 0;
    while open < len {
        match bytes[open] {
            b'[' => {
                if depth == 0 {
                    // `\s*`
                    let mut after_ws = open + 1;
                    while after_ws < len && bytes[after_ws].is_ascii_whitespace() {
                        after_ws += 1;
                    }
                    // `expr` followed by a whitespace byte.
                    let kw_end = after_ws + 4;
                    if kw_end < len
                        && &bytes[after_ws..kw_end] == b"expr"
                        && bytes[kw_end].is_ascii_whitespace()
                    {
                        // Depth-scan for the matching `]` (the open `[` is
                        // already counted).
                        let mut d = 1;
                        let mut scan = open + 1;
                        while scan < len && d > 0 {
                            match bytes[scan] {
                                b'[' => d += 1,
                                b']' => d -= 1,
                                _ => {}
                            }
                            scan += 1;
                        }
                        let close = if d == 0 { scan - 1 } else { len - 1 };
                        return Some((open, close));
                    }
                }
                depth += 1;
            }
            b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        open += 1;
    }
    None
}

/// The W114 unwrap fix: replace the nested `[expr {INNER}]` (at
/// `open..=close` within `outer`, absolute span `span`) with
/// `(INNER)` — or bare `INNER` when it is a single atom.  `None`
/// unless every condition for a purely textual inline holds:
///
/// * the outer argument is braced (`outer` starts with `{`), so no
///   Tcl-level substitution reruns over the inlined text;
/// * the bracket scan really matched (`outer[close]` is `]`);
/// * the outer expression carries no string-comparison operator
///   (`eq`/`ne`/`in`/`ni`): a nested `expr` normalises a numeric
///   result (`[expr {$x}]` yields `7` for `x == "007"`), so unwrapping
///   could flip a string comparison's verdict;
/// * the nested body is exactly one braced group — an unbraced body
fn string_comparison_op_spellings() -> &'static [&'static str] {
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, OperatorShape};
    static OPS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                let is_string_family = matches!(
                    spec.mathop_shape,
                    Some(
                        OperatorShape::BoolChain { string_only: true }
                            | OperatorShape::NeBinary { string_only: true }
                            | OperatorShape::Membership { .. }
                    )
                );
                is_string_family.then_some(spec.spelling)
            })
            .collect()
    })
}

/// True when `text` contains a standalone word from
/// [`string_comparison_op_spellings`] — the expr string-comparison
/// operators whose verdict a nested `expr`'s numeric normalisation can
/// influence.  Deliberately scans the whole outer expression (a coarse,
/// conservative over-match).
fn contains_string_comparison_word(text: &str) -> bool {
    let bytes = text.as_bytes();
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$';
    for op in string_comparison_op_spellings() {
        let mut from = 0;
        while let Some(pos) = text.get(from..).and_then(|t| t.find(op)) {
            let at = from + pos;
            let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
            let after = at + op.len();
            let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
            from = at + 1;
        }
    }
    false
}

/// Whole scalar references use the original source grammar; the literal arm
/// only preserves the existing conservative parenthesis-elision eligibility.
pub(super) fn is_expr_atom(inner: &str, config: tcl_lexer::LexerConfig) -> bool {
    tcl_syntax::naming::whole_scalar_variable_reference_name(inner, config).is_some()
        || (!inner.is_empty()
            && inner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_')))
}

/// Walk `node` and collect every `==`/`!=` operator that compares as strings
/// whatever its other operand holds ([`eq_ne_compares_as_strings`]), paired with
/// the operator's byte offset within `text` (the parsed expression
/// source) when it can be located — `None` when an operand extent is
/// unavailable (a `Raw` child) or the operator text is not found in the
/// gap between the operands.
///
/// Comparisons between
/// two variables (`$x == $y`) are intentionally *not* collected —
/// the variables may hold integer values, making `==` correct.
#[cfg(test)]
pub(super) fn find_string_eq_ne_ops(node: &ExprNode, _text: &str) -> Vec<(BinOp, Option<u32>)> {
    let mut found = Vec::new();
    walk_string_eq_ne(node, None, &mut found, 0);
    found
}

fn find_original_string_eq_ne_ops(
    node: &ExprNode,
    original: &tcl_syntax::expr::substitution::OriginalSourceExpression,
) -> Vec<(BinOp, Option<u32>)> {
    let mut found = Vec::new();
    walk_string_eq_ne(node, Some(original), &mut found, 0);
    found
}

pub(super) fn walk_string_eq_ne(
    node: &ExprNode,
    original: Option<&tcl_syntax::expr::substitution::OriginalSourceExpression>,
    found: &mut Vec<(BinOp, Option<u32>)>,
    depth: u32,
) {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the `==`/`!=`-against-string findings gathered so far is
    // the safe fallback (occurrences buried deeper than the cap go
    // unreported; never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_string_eq_ne(left, original, found, depth + 1);
            walk_string_eq_ne(right, original, found, depth + 1);
            if matches!(op, BinOp::Eq | BinOp::Ne) && eq_ne_compares_as_strings(left, right) {
                let span = original.and_then(|owner| owner.binary_operator_span(node));
                found.push((*op, span.map(tcl_lexer::Span::start)));
            }
        }
        ExprNode::Unary { operand, .. } => walk_string_eq_ne(operand, original, found, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_string_eq_ne(condition, original, found, depth + 1);
            walk_string_eq_ne(true_branch, original, found, depth + 1);
            walk_string_eq_ne(false_branch, original, found, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_string_eq_ne(arg, original, found, depth + 1);
            }
        }
        _ => {}
    }
}

/// The `(start, end)` extent of `node` within its parsed text, from its
/// leaves' offsets (`end` inclusive — the expr lexer's convention).
/// `None` when a `Raw` child makes the extent unknowable.
#[cfg(test)]
pub(super) fn node_extent(node: &ExprNode) -> Option<(u32, u32)> {
    // Entry point: the top of an expression tree is nesting depth 0 (the
    // recursion cap is retained by the shared AST geometry owner).
    node.lexical_source_extent()
}
