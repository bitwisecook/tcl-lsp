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

//! O128 — end-offset index rewrite.
//!
//! Rewrites length arithmetic used as a list/string index to Tcl's
//! `end` / `end-N` form: `lindex $L [expr {[llength $L] - 1}]` →
//! `lindex $L end`, `string range $s 0 [expr {[string length $s] - 2}]`
//! → `string range $s 0 end-1`, and so on. The rewrite only fires when
//! the length command's argument is *textually identical* to the
//! command's own list/string operand: rewriting `[llength $A]` used to
//! index `$B` would change semantics.
//!
//! Candidates are advisory until an exact repeated-read contents witness,
//! native handler/compiler proofs and a reached expression preparation prove
//! the original and replacement schedules equivalent. The initial executable
//! case is one `lindex` index into a known valid list with an in-bounds offset.

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, TokenType};

use crate::compilation_unit::CompilationUnit;
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{Script, Statement};
use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};

use super::{Optimisation, PassContext};

/// Which length builtin produced an end-offset candidate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LengthKind {
    /// `llength` — list length.
    Llength,
    /// `string length` — string length.
    Strlen,
}

/// Actual original syntax and metadata supplied by this compilation module.
/// Missing metadata cannot acquire executable permission from hint spelling.
#[derive(Clone, Copy)]
struct EndOffsetSource<'a> {
    tokens: &'a crate::ir::CommandTokens,
    script: &'a Script,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    parser: Option<tcl_syntax::expr::parser::ExprParseContext>,
}

impl EndOffsetSource<'_> {
    fn command_tokens(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<crate::ir::CommandTokens> {
        let tokens = if self.tokens.argv.first() == command.argv.first().map(|word| &word.span) {
            self.tokens.clone()
        } else {
            let mut nested = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                self.config,
                command,
            );
            nested.inherit_nested_bindings(self.tokens);
            nested
        };
        (tokens
            .source_binding
            .as_ref()?
            .source_origin()?
            .source_image()
            == &tcl_lexer::SourceImage::document(source))
            .then_some(tokens)
    }

    fn shape(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<crate::registry_invocation::RegistryInvocationShape> {
        let metadata = self
            .metadata
            .filter(|metadata| metadata.matches_registry(registry))?;
        let assistance = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry, Some(metadata), tokens,
        )?;
        assistance.unanimous_command_words()?;
        let first = assistance.candidates.first()?;
        assistance
            .candidates
            .iter()
            .all(|candidate| {
                candidate.operation == first.operation
                    && candidate.argument_offset == first.argument_offset
            })
            .then(|| first.clone())
    }
}

/// Run the O128 end-offset detection over the whole compilation unit.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // Copy the source reference so the immutable slice borrow does not
    // conflict with the `&mut ctx` report calls below.
    let source = ctx.source;
    // The document's own grammar, threaded from the pass context's resolved
    // profile — the segmenter re-reads statement source here.
    let config = cu.ir_module.lexer_config;
    let actual = ctx.registry.and_then(|registry| {
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module)
    });
    let metadata = actual.as_deref().map(Into::into);
    let parser = cu.ir_module.source_metadata_input.as_ref().map(|input| {
        let mut parser =
            tcl_syntax::expr::parser::ExprParseContext::for_profile(input.unit_profile());
        parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
        parser
    });
    let mut spans = Vec::new();
    collect_statement_spans(&cu.ir_module.top_level, &mut spans, 0);
    for proc in cu.ir_module.procedures.values() {
        collect_statement_spans(&proc.body, &mut spans, 0);
    }
    for (span, script, tokens) in spans {
        let Some(slice) = source.get(span.start() as usize..span.end() as usize) else {
            continue;
        };
        for cmd in segment_commands_with_offset_and_config(slice, span.start(), config) {
            apply_to_command(
                ctx,
                &cmd,
                0,
                Some(EndOffsetSource {
                    tokens,
                    script,
                    config,
                    metadata,
                    parser,
                }),
            );
        }
    }
}

/// Collect every statement's span, recursing through control-flow bodies
/// so commands nested in `if` / `for` / `while` / `switch` / `try`
/// bodies are reached. `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn collect_statement_spans<'a>(
    script: &'a Script,
    out: &mut Vec<(Span, &'a Script, &'a crate::ir::CommandTokens)>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        if let Some(span) = stmt.source_edit_span()
            && let Some(tokens) = stmt.tokens()
        {
            out.push((span, script, tokens));
        }
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    collect_statement_spans(&c.body, out, depth + 1);
                }
                if let Some(b) = else_body {
                    collect_statement_spans(b, out, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_statement_spans(init, out, depth + 1);
                collect_statement_spans(next, out, depth + 1);
                collect_statement_spans(body, out, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => collect_statement_spans(body, out, depth + 1),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_statement_spans(body, out, depth + 1);
                for h in handlers {
                    collect_statement_spans(&h.body, out, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_statement_spans(fb, out, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_statement_spans(b, out, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    collect_statement_spans(b, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Emit O128 for any end-offset index in `cmd`, then recurse into the
/// command's nested `[…]` substitutions.
fn apply_to_command(
    ctx: &mut PassContext<'_>,
    cmd: &SegmentedCommand,
    depth: u32,
    proof: Option<EndOffsetSource<'_>>,
) {
    // Native-stack safety net: recurses into nested `[…]`
    // substitutions inside a command word, a genuinely unbounded axis. Past
    // the cap, stop descending — the only effect is that O128 opportunities
    // buried deeper than the cap go unreported; never a crash.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    emit_for_command(ctx, cmd, proof);
    for (idx, tok) in cmd.argv.iter().enumerate() {
        if cmd.single_token_word.get(idx).copied() != Some(true) {
            continue;
        }
        if tok.kind != TokenType::Cmd {
            continue;
        }
        // The word text is the verbatim `[inner]`; re-segment `inner`
        // anchored one byte past the opening `[` so the recovered spans
        // stay absolute.
        let full = &cmd.texts[idx];
        let Some(inner) = full.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        let base = tok.span.start() + 1;
        for nested in segment_commands_with_offset_and_config(
            inner,
            base,
            proof.map_or_else(
                || tcl_lexer::LexerConfig::for_profile(ctx.dialect),
                |proof| proof.config,
            ),
        ) {
            apply_to_command(ctx, &nested, depth + 1, proof);
        }
    }
}

/// Emit O128 for each index argument of `cmd` that is length arithmetic
/// over the command's own list/string operand.
fn emit_for_command(
    ctx: &mut PassContext<'_>,
    cmd: &SegmentedCommand,
    proof: Option<EndOffsetSource<'_>>,
) {
    let Some(source) = proof else {
        return;
    };
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(tokens) = source.command_tokens(ctx.source, cmd) else {
        return;
    };
    let Some(candidate) = source.shape(registry, &tokens) else {
        return;
    };
    let Some((positions, container, expected)) = end_offset_command_shape(&candidate) else {
        return;
    };
    let Some(container_position) = written_word(&candidate, container) else {
        return;
    };
    let Some(container) = cmd.argv.get(container_position) else {
        return;
    };
    if cmd.single_token_word.get(container_position).copied() != Some(true)
        || container.kind != TokenType::Var
    {
        return;
    }
    let Some(container_repr) = cmd.texts.get(container_position) else {
        return;
    };
    for position in positions {
        let Some(written) = written_word(&candidate, position) else {
            continue;
        };
        let Some(index) = cmd.argv.get(written) else {
            continue;
        };
        if cmd.single_token_word.get(written).copied() != Some(true) || index.kind != TokenType::Cmd
        {
            continue;
        }
        let Some((kind, length_argument, offset)) =
            try_end_offset_from_length_expr(ctx.source, registry, source, &tokens, written)
        else {
            continue;
        };
        if kind != expected || length_argument.trim() != container_repr.trim() {
            continue;
        }
        let replacement = if offset == 0 {
            "end".to_owned()
        } else {
            format!("end-{offset}")
        };
        let span = tcl_lexer::word_span_at(ctx.source, index.span);
        let mut suggestion = Optimisation::new(
            DiagCode::O128,
            "Use end-offset index instead of length arithmetic",
            span,
            replacement.clone(),
        );
        // Source shape and spelling only justify a hint. Independent original
        // reads, native handlers and preparation still own applicability.
        suggestion.hint_only = !closed_source_candidate(ctx, cmd, proof, &replacement);
        ctx.report(suggestion);
    }
}

fn closed_source_candidate(
    ctx: &PassContext<'_>,
    cmd: &SegmentedCommand,
    proof: Option<EndOffsetSource<'_>>,
    replacement: &str,
) -> bool {
    proof.is_some_and(|source| {
        let Some(metadata) = source.metadata else {
            return false;
        };
        let Some(registry) = ctx.registry else {
            return false;
        };
        let Some(tokens) = source.command_tokens(ctx.source, cmd) else {
            return false;
        };
        closed_list_end_offset(
            &tokens,
            source.script,
            registry,
            metadata,
            source.config,
            replacement,
        )
        .is_some()
    })
}

/// Closed initial case: one list index, one native length read, decimal
/// subtraction, and two unchanged observer-free reads of a known valid list.
fn closed_list_end_offset(
    tokens: &crate::ir::CommandTokens,
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    config: tcl_lexer::LexerConfig,
    replacement: &str,
) -> Option<()> {
    use crate::registry_invocation::resolved_tokens_invocation_with_metadata_context;
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let outer = resolved_tokens_invocation_with_metadata_context(registry, Some(metadata), tokens)?;
    if outer.facts.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)
        || tokens.words().len() != 3
        || !outer.effective.binding_prefix.is_empty()
        || outer.facts.effects.requires_world_barrier()
    {
        return None;
    }
    let (_, first_read) = tokens.words()[1].sole_variable_substitution()?;
    let (spelling, site) = tokens.words()[2].sole_command_substitution()?;
    let mut expression = crate::word_subst::nested_command_words(spelling, site, config).ok()?;
    expression.inherit_nested_bindings(tokens);
    let selected =
        resolved_tokens_invocation_with_metadata_context(registry, Some(metadata), &expression)?;
    if selected.facts.operation
        != SemanticOperationId::StructuredLowering(tcl_registry::hooks::LoweringHookId::Expr)
        || expression.words().len() != 2
        || !selected.effective.binding_prefix.is_empty()
    {
        return None;
    }
    // The prepared tree below bounds every reached expression operand. The
    // generic expr footprint includes arbitrary nested scripts; this exact
    // tree instead has one independently proved callback-free length call.
    let (length, subtraction, dialect) = prepared_length_subtraction(
        tokens,
        &expression,
        script,
        registry,
        config,
        selected.dialect?,
    )?;
    let length_call =
        resolved_tokens_invocation_with_metadata_context(registry, Some(metadata), &length)?;
    if length_call.facts.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
        || length.words().len() != 2
        || !length_call.effective.binding_prefix.is_empty()
        || length_call.facts.effects.requires_world_barrier()
    {
        return None;
    }
    let (_, second_read) = length.words()[1].sole_variable_substitution()?;
    let reads = crate::read_schedule::RepeatedReadContentsWitness::prove(
        tokens,
        first_read,
        second_read,
        registry,
    )?;
    if reads.dialect() != dialect {
        return None;
    }
    let elements = dialect.word_values.split_list(reads.value()).ok()?;
    // Both retained native handlers interpret this same object as a list.
    // The original length call installs/reads that representation before
    // lindex; the rewrite retains lindex's interpretation. Initial object
    // representation is therefore not a freshness or string-type premise.
    // No observer or callback may run between the two proved original reads.
    let element_count = i32::try_from(elements.len()).ok()?;
    let original = element_count.checked_sub(subtraction)?;
    // Bounds keep every engine's signed index representation closed, including Jim.
    if original < 0 || original >= element_count {
        return None;
    }
    let rewritten =
        tcl_cmd_core::index::resolve_opt_in(replacement, elements.len(), dialect.index_syntax()?)?;
    (rewritten == i64::from(original)).then_some(())
}

fn prepared_length_subtraction(
    tokens: &crate::ir::CommandTokens,
    expression: &crate::ir::CommandTokens,
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
) -> Option<(
    crate::ir::CommandTokens,
    i32,
    tcl_registry::InvocationDialect,
)> {
    let prepared = crate::math_function_binding::ExpressionMathBindings::new(script, None)
        .at_invocation(expression.words().first()?.source().span.start())?;
    let context =
        crate::tcl_expr_eval::FoldPolicy::for_retained_entry(registry, Some(dialect), &config)
            .preparation_context()?;
    let preparation = prepared.preparation_for_context(&context)?;
    if preparation.source.origin != script.executed_source.as_ref()?.origin {
        return None;
    }
    let ExprNode::Binary {
        op: BinOp::Sub,
        left,
        right,
    } = preparation.witness.tree()
    else {
        return None;
    };
    let ExprNode::Command { text, start, end } = left.as_ref() else {
        return None;
    };
    let ExprNode::Literal {
        text: subtraction, ..
    } = right.as_ref()
    else {
        return None;
    };
    if !subtraction
        .as_bytes()
        .first()
        .is_some_and(|digit| matches!(digit, b'1'..=b'9'))
        || !subtraction.bytes().all(|digit| digit.is_ascii_digit())
    {
        return None;
    }
    let subtraction = subtraction.parse::<i32>().ok()?;
    let base = preparation.source.base();
    let site = crate::ir::SourceSite::source(Span::new(
        base.checked_add(*start)?,
        base.checked_add(*end)?,
    ));
    let mut length = crate::word_subst::nested_command_words(text, &site, config).ok()?;
    length.inherit_nested_bindings(tokens);
    Some((length, subtraction, dialect))
}

/// Original effective operand positions for a Registry-selected operation.
/// Insertion end semantics are different; later multi-index operands operate
/// on sublists. Neither can inherit the first container's end coordinate.
fn end_offset_command_shape(
    shape: &crate::registry_invocation::RegistryInvocationShape,
) -> Option<(Vec<usize>, usize, LengthKind)> {
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let SemanticOperationId::Intrinsic(operation) = shape.operation else {
        return None;
    };
    let container = shape.argument_offset.checked_add(1)?;
    let count = shape.effective.words.len().checked_sub(container)?;
    let first = container.checked_add(1)?;
    let last = first.checked_add(1)?;
    match operation {
        IntrinsicId::ListIndex if count >= 2 => Some((vec![first], container, LengthKind::Llength)),
        IntrinsicId::ListRange if count == 3 => {
            Some((vec![first, last], container, LengthKind::Llength))
        }
        IntrinsicId::ListReplace if count >= 3 => {
            Some((vec![first, last], container, LengthKind::Llength))
        }
        IntrinsicId::StringIndex if count == 2 => {
            Some((vec![first], container, LengthKind::Strlen))
        }
        IntrinsicId::StringRange if count == 3 => {
            Some((vec![first, last], container, LengthKind::Strlen))
        }
        IntrinsicId::StringReplace if count >= 3 => {
            Some((vec![first, last], container, LengthKind::Strlen))
        }
        _ => None,
    }
}

fn written_word(
    shape: &crate::registry_invocation::RegistryInvocationShape,
    index: usize,
) -> Option<usize> {
    match shape.effective.origins.get(index)? {
        crate::registry_invocation::InvocationWordOrigin::Written(index) => Some(*index),
        _ => None,
    }
}

/// A sole original expression operand with unchanged source bytes and scope.
/// Captured/expanded body values lack this direct mapping and are refused.
fn original_index_expression(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    advice: EndOffsetSource<'_>,
    parent: &crate::ir::CommandTokens,
    index: usize,
) -> Option<(crate::ir::CommandTokens, ExprNode, u32)> {
    let (spelling, site) = parent.words().get(index)?.sole_command_substitution()?;
    let mut expression =
        crate::word_subst::nested_command_words(spelling, site, advice.config).ok()?;
    expression.inherit_nested_bindings(parent);
    let shape = advice.shape(registry, &expression)?;
    if shape.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    {
        return None;
    }
    let operand = shape.argument_offset.checked_add(1)?;
    (shape.effective.words.len() == operand.checked_add(1)?).then_some(())?;
    let written = written_word(&shape, operand)?;
    let (crate::ir::WordExpr::BracedLiteral { text, .. }
    | crate::ir::WordExpr::Literal { text, .. }) = expression.words().get(written)?
    else {
        return None;
    };
    let base = crate::lowering_hooks::word_content_base(
        *expression.argv.get(written)?,
        *expression.single_token_word.get(written)?,
        text,
    )?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    (source.as_bytes().get(base as usize..end as usize) == Some(text.as_bytes())).then_some(())?;
    let node = crate::expr_parser::parse_expr_with_syntax_context(text, &advice.parser?);
    Some((expression, node, base))
}

/// Typed original length operation and decimal subtraction topology supply
/// advisory end-offset syntax. Native expression preparation remains separate.
fn try_end_offset_from_length_expr(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    advice: EndOffsetSource<'_>,
    parent: &crate::ir::CommandTokens,
    index: usize,
) -> Option<(LengthKind, String, i64)> {
    let (expression, node, base) =
        original_index_expression(source, registry, advice, parent, index)?;
    let ExprNode::Binary {
        op: BinOp::Sub,
        left,
        right,
    } = node
    else {
        return None;
    };
    let ExprNode::Command { text, start, end } = left.as_ref() else {
        return None;
    };
    let ExprNode::Literal {
        text: subtraction, ..
    } = right.as_ref()
    else {
        return None;
    };
    let subtraction = subtraction.trim();
    if !subtraction
        .as_bytes()
        .first()
        .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
        || !subtraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let offset = subtraction.parse::<i64>().ok()?.checked_sub(1)?;
    let site = crate::ir::SourceSite::source(Span::new(
        base.checked_add(*start)?,
        base.checked_add(*end)?,
    ));
    let mut length = crate::word_subst::nested_command_words(text, &site, advice.config).ok()?;
    length.inherit_nested_bindings(&expression);
    let shape = advice.shape(registry, &length)?;
    let kind = match shape.operation {
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ListLength) => {
            LengthKind::Llength
        }
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::StringLength) => {
            LengthKind::Strlen
        }
        _ => return None,
    };
    let argument = shape.argument_offset.checked_add(1)?;
    (shape.effective.words.len() == argument.checked_add(1)?).then_some(())?;
    let written = written_word(&shape, argument)?;
    let (spelling, _) = length.words().get(written)?.sole_variable_substitution()?;
    Some((kind, spelling.to_owned(), offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interprocedural::InterproceduralAnalysis;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// The replacement and the exact source slice the rewrite targets.
    fn o128_rewrite(source: &str) -> Option<(String, String)> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        let opt = ctx
            .optimisations
            .into_iter()
            .find(|o| o.code == DiagCode::O128)?;
        let slice = source
            .get(opt.span.start() as usize..opt.span.end() as usize)?
            .to_owned();
        Some((slice, opt.replacement))
    }

    /// `collect_statement_spans` recurses once per nested
    /// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` body, so it needs a
    /// depth cap of its own. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by the lowering pass, so this is defence-in-depth / consistency
    /// with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 1000 levels of source nesting is
    /// comfortably past that cap; the assertion is that `run_pass`
    /// returns at all, not what it returns. Spawns its own big-stack
    /// thread since the lexer/CST/segmenter stages upstream of the
    /// lowering cap still walk the full un-truncated source nesting before
    /// that cap trims it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_collect_statement_spans() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("lindex $L [expr {[llength $L] - 1}]\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = run_pass(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// `apply_to_command` recurses once
    /// per nested `[cmd …]` substitution inside a single command word (Tier
    /// 1B) — a genuinely unbounded axis, independent of the statement-tree
    /// nesting cap. Uncapped it overflows the native stack (SIGABRT) in
    /// the low thousands of levels on a 2 MiB thread. Segment a
    /// `foo [a [a [… [x] …]]]` word directly and drive the pass on it (the
    /// nesting lives in one argument word, so it never reaches lowering's
    /// statement-tree cap). 3000 is past the crash range and past
    /// `MAX_BRACKET_TEXT_DEPTH` (256); the assertion is that it returns.
    #[test]
    fn deeply_nested_apply_to_command_survives() {
        let reg = registry();
        let mut deep = "x".to_owned();
        for _ in 0..3000 {
            deep = format!("a [{deep}]");
        }
        let source = format!("foo [{deep}]");
        let mut ctx = PassContext::new(&source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        for cmd in
            segment_commands_with_offset_and_config(&source, 0, tcl_lexer::LexerConfig::default())
        {
            apply_to_command(&mut ctx, &cmd, 0, None);
        }
    }

    #[test]
    fn original_end_offset_hints_use_typed_targets_and_written_operand_origins() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        for source in [
            "lindex $L [expr {[llength $L] - 1}]",
            "rename lindex pick; rename llength size; rename expr calculate; pick $L [calculate {[size $L] - 1}]",
            "interp alias {} pick {} lindex; interp alias {} size {} llength; pick $L [expr {[size $L] - 1}]",
            "interp alias {} chars {} string length; interp alias {} character {} string index; character $s [expr {[chars $s] - 2}]",
        ] {
            let candidates = run_pass(source);
            assert_eq!(
                candidates
                    .iter()
                    .filter(|candidate| candidate.code == DiagCode::O128)
                    .count(),
                1,
                "{source}: {candidates:?}"
            );
        }
        for source in [
            "proc lindex args {return CUSTOM}; lindex $L [expr {[llength $L] - 1}]",
            "proc llength args {return CUSTOM}; lindex $L [expr {[llength $L] - 1}]",
            "proc expr args {return CUSTOM}; lindex $L [expr {[llength $L] - 1}]",
            "rename llength {}; lindex $L [expr {[llength $L] - 1}]",
            "interp alias {} picked {} lindex CAPTURED; picked [expr {[llength $L] - 1}]",
            "lindex $L [expr {[llength $L] - 010}]",
        ] {
            assert!(
                run_pass(source)
                    .iter()
                    .all(|candidate| candidate.code != DiagCode::O128),
                "{source}"
            );
        }
    }

    #[test]
    fn end_offset_applicability_requires_actual_module_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Actual original registrations are retained independently of metadata
        // availability. The owner stays alive through every refusal control.
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut catalogue = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut descriptor = catalogue.get("lindex").unwrap().clone();
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        catalogue.insert(descriptor);
        let current =
            std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)));
        let registry = current.commands();
        let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(
            registry.profile().unwrap(),
        );
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            native_entry: Some(std::sync::Arc::new(native)),
            ..crate::command_binding::SourceAnalysisEntry::default()
        };
        let mut unit = CompilationUnit::build_with_context_registry(
            "set L {a b c}; lindex $L [expr {[llength $L] - 2}]",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&current),
        );
        let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
        context.registry = Some(registry);
        context.dialect = registry.profile();
        run(&mut context, &unit);
        assert!(
            context
                .optimisations
                .iter()
                .any(|candidate| candidate.code == DiagCode::O128 && !candidate.hint_only)
        );
        let original = unit.ir_module.top_level.clone();
        let input = unit.ir_module.source_metadata_input.clone().unwrap();
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(registry)),
        );
        assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        for withheld in [Some(older), Some(foreign), None] {
            unit.ir_module.source_metadata_input = withheld.map(|availability| {
                crate::analyser::ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    availability,
                    input.lexer_config(),
                )
            });
            let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
            context.registry = Some(registry);
            context.dialect = registry.profile();
            run(&mut context, &unit);
            assert!(
                context
                    .optimisations
                    .iter()
                    .filter(|candidate| candidate.code == DiagCode::O128)
                    .all(|candidate| candidate.hint_only)
            );
            assert_eq!(unit.ir_module.top_level, original);
        }
    }

    #[test]
    fn end_offset_candidates_do_not_erase_read_trace_schedules() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let source = "set L {a b}; proc observe args {}; trace add variable L read observe; lindex $L [expr {[llength $L] - 1}]";
        let cu = CompilationUnit::build_for(source, registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        run(&mut ctx, &cu);
        let candidates: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|candidate| candidate.code == DiagCode::O128)
            .collect();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].hint_only);
    }

    #[test]
    fn closed_known_list_schedule_can_erase_only_the_redundant_read() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let source = "set L {a b c}; lindex $L [expr {[llength $L] - 2}]";
            let cu = CompilationUnit::build_for(source, registry, false);
            let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
            ctx.registry = Some(registry);
            ctx.dialect = registry.profile();
            run(&mut ctx, &cu);
            let candidate = ctx
                .optimisations
                .iter()
                .find(|candidate| candidate.code == DiagCode::O128)
                .unwrap();
            assert!(
                !candidate.hint_only,
                "{profile}: {candidate:?}; preparations={:?}; reads={:?}",
                cu.ir_module
                    .top_level
                    .expression_preparations
                    .iter()
                    .map(|proof| {
                        (
                            proof.invocation.offset,
                            proof.source.base(),
                            proof.source.text.as_ref(),
                            proof.witness.tree(),
                        )
                    })
                    .collect::<Vec<_>>(),
                cu.ir_module
                    .top_level
                    .statements
                    .last()
                    .unwrap()
                    .tokens()
                    .unwrap()
                    .variable_accesses
                    .iter()
                    .map(|read| {
                        (
                            &read.original_spelling,
                            read.source.span,
                            read.context_residual(),
                            read.context_alternatives().len(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            assert_eq!(candidate.replacement, "end-1");
        }
    }

    #[test]
    fn closed_schedule_retains_list_conversion_for_a_shared_numeric_input() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let owner = tcl_registry::model::ingress::static_context_for(profile);
            let registry = owner.commands();
            let source = "set L [expr {2 + 2}]; set alias $L; lindex $L [expr {[llength $L] - 1}]; concat $alias [list]";
            let cu = CompilationUnit::build_for(source, registry, false);
            let mut context = PassContext::new(&cu.source, InterproceduralAnalysis::default());
            context.registry = Some(registry);
            context.dialect = registry.profile();
            run(&mut context, &cu);
            let candidate = context
                .optimisations
                .iter()
                .find(|candidate| candidate.code == DiagCode::O128)
                .unwrap();
            assert!(!candidate.hint_only, "{profile}: {candidate:?}");
            assert_eq!(candidate.replacement, "end");
        }
    }

    #[test]
    fn repeated_read_rewrite_declines_unknown_invalid_and_replaced_schedules() {
        for source in [
            "lindex $L [expr {[llength $L] - 1}]",
            r"set L \{; lindex $L [expr {[llength $L] - 1}]",
            "set L {a b}; proc llength args {return 2}; lindex $L [expr {[llength $L] - 1}]",
            "set L {a b}; proc expr args {return 1}; lindex $L [expr {[llength $L] - 1}]",
            "set L {a b}; lindex $L [expr {[llength $L] - 3}]",
        ] {
            assert!(
                run_pass(source)
                    .iter()
                    .filter(|candidate| candidate.code == DiagCode::O128)
                    .all(|candidate| candidate.hint_only),
                "{source}"
            );
        }
    }

    #[test]
    fn lindex_last_element_rewrites_to_end() {
        let (slice, repl) = o128_rewrite("lindex $L [expr {[llength $L] - 1}]").unwrap();
        assert_eq!(slice, "[expr {[llength $L] - 1}]");
        assert_eq!(repl, "end");
    }

    #[test]
    fn lindex_offset_two_rewrites_to_end_minus_one() {
        let (_slice, repl) = o128_rewrite("lindex $L [expr {[llength $L] - 2}]").unwrap();
        assert_eq!(repl, "end-1");
    }

    #[test]
    fn string_range_last_char_rewrites_to_end() {
        let (slice, repl) =
            o128_rewrite("string range $s 0 [expr {[string length $s] - 1}]").unwrap();
        assert_eq!(slice, "[expr {[string length $s] - 1}]");
        assert_eq!(repl, "end");
    }

    #[test]
    fn lrange_rewrites_both_index_positions() {
        let opts = run_pass("lrange $L [expr {[llength $L] - 3}] [expr {[llength $L] - 1}]");
        let repls: Vec<&str> = opts
            .iter()
            .filter(|o| o.code == DiagCode::O128)
            .map(|o| o.replacement.as_str())
            .collect();
        assert!(repls.contains(&"end-2"), "got {repls:?}");
        assert!(repls.contains(&"end"), "got {repls:?}");
    }

    #[test]
    fn mismatched_container_does_not_rewrite() {
        // The length is of `$A` but the indexed list is `$L` — rewriting
        // to `end` would change semantics.
        let opts = run_pass("lindex $L [expr {[llength $A] - 1}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "mismatched container must not rewrite, got {opts:?}",
        );
    }

    #[test]
    fn bare_llength_index_is_not_end_offset() {
        // `[llength $L]` (no subtraction) is one past the last index.
        let opts = run_pass("lindex $L [expr {[llength $L]}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "got {opts:?}"
        );
    }

    #[test]
    fn linsert_is_excluded() {
        let opts = run_pass("linsert $L [expr {[llength $L] - 1}] x");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "got {opts:?}"
        );
    }

    #[test]
    fn rewrites_inside_proc_body() {
        let (_slice, repl) =
            o128_rewrite("proc ::f {L} { return [lindex $L [expr {[llength $L] - 1}]] }").unwrap();
        assert_eq!(repl, "end");
    }
}
