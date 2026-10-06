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
use crate::expr_parser::parse_expr_for_profile;
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

/// Run the O128 end-offset detection over the whole compilation unit.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // Copy the source reference so the immutable slice borrow does not
    // conflict with the `&mut ctx` report calls below.
    let source = ctx.source;
    // The document's own grammar, threaded from the pass context's resolved
    // profile — the segmenter re-reads statement source here.
    let config = cu.ir_module.lexer_config;
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
            apply_to_command(ctx, &cmd, 0, Some((tokens, script, config)));
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
    proof: Option<(&crate::ir::CommandTokens, &Script, tcl_lexer::LexerConfig)>,
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
                |(_, _, config)| config,
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
    proof: Option<(&crate::ir::CommandTokens, &Script, tcl_lexer::LexerConfig)>,
) {
    let Some((index_positions, container_pos, expected)) = end_offset_command_shape(&cmd.texts)
    else {
        return;
    };
    if container_pos >= cmd.texts.len() || container_pos >= cmd.argv.len() {
        return;
    }
    if cmd.single_token_word.get(container_pos).copied() != Some(true) {
        return;
    }
    if cmd.argv[container_pos].kind != TokenType::Var {
        return;
    }
    // Compare full variable references (`${L}`, `$a(1)`), not normalised
    // base names — `$a(1)` and `$a(2)` are different containers.
    let container_repr = cmd.texts[container_pos].trim();

    for pos in index_positions {
        if pos >= cmd.texts.len() || pos >= cmd.argv.len() {
            continue;
        }
        if cmd.single_token_word.get(pos).copied() != Some(true) {
            continue;
        }
        let idx_tok = &cmd.argv[pos];
        if idx_tok.kind != TokenType::Cmd {
            continue;
        }
        let Some(expr_arg) = expr_arg_from_command_word(
            &cmd.texts[pos],
            ctx.registry,
            tcl_lexer::LexerConfig::for_profile(ctx.dialect),
        ) else {
            continue;
        };
        let Some((kind, length_arg, offset)) =
            try_end_offset_from_length_expr(&expr_arg, ctx.dialect)
        else {
            continue;
        };
        if kind != expected || length_arg.trim() != container_repr {
            continue;
        }
        let replacement = if offset == 0 {
            "end".to_owned()
        } else {
            format!("end-{offset}")
        };
        // The `Cmd` token span follows the lexer's inner-end convention, so
        // the full `[…]` substitution needs its closing `]`. Deriving that
        // as `end + 1` overshoots an empty `[]`, so it goes through the
        // owner rather than by hand.
        let span = tcl_lexer::word_span_at(ctx.source, idx_tok.span);
        let mut suggestion = Optimisation::new(
            DiagCode::O128,
            "Use end-offset index instead of length arithmetic",
            span,
            replacement.clone(),
        );
        // Matching spelling does not prove that two original reads see the
        // same object or that removing the length call preserves its effects.
        suggestion.hint_only = !closed_source_candidate(ctx, cmd, proof, &replacement);

        ctx.report(suggestion);
    }
}

fn closed_source_candidate(
    ctx: &PassContext<'_>,
    cmd: &SegmentedCommand,
    proof: Option<(&crate::ir::CommandTokens, &Script, tcl_lexer::LexerConfig)>,
    replacement: &str,
) -> bool {
    proof.is_some_and(|(parent, script, config)| {
        let Some(registry) = ctx.registry else {
            return false;
        };
        let tokens = if parent.argv.first() == cmd.argv.first().map(|word| &word.span) {
            parent.clone()
        } else {
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(ctx.source),
                config,
                cmd,
            );
            tokens.inherit_nested_bindings(parent);
            tokens
        };
        closed_list_end_offset(&tokens, script, registry, config, replacement).is_some()
    })
}

/// Closed initial case: one list index, one native length read, decimal
/// subtraction, and two unchanged observer-free reads of a known valid list.
fn closed_list_end_offset(
    tokens: &crate::ir::CommandTokens,
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    replacement: &str,
) -> Option<()> {
    use crate::registry_invocation::resolved_tokens_invocation;
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let outer = resolved_tokens_invocation(registry, None, tokens)?;
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
    let selected = resolved_tokens_invocation(registry, None, &expression)?;
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
    let length_call = resolved_tokens_invocation(registry, None, &length)?;
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

/// Identify list/string commands that accept `end` / `end-N` index args.
/// Returns `(index_positions, container_position, expected_kind)`.
///
/// `linsert` is intentionally excluded (`linsert $L end x` appends, but
/// `linsert $L [expr {[llength $L] - 1}] x` inserts before the last
/// element — no `end`/`end-N` rewrite preserves that). `lindex` with
/// multiple indices resolves later indices against sub-lists, so only the
/// first index (position 2) is safe.
fn end_offset_command_shape(texts: &[String]) -> Option<(Vec<usize>, usize, LengthKind)> {
    let cmd = texts.first()?.as_str();
    let nargs = texts.len();
    match cmd {
        "lindex" if nargs >= 3 => Some((vec![2], 1, LengthKind::Llength)),
        "lrange" if nargs == 4 => Some((vec![2, 3], 1, LengthKind::Llength)),
        "lreplace" if nargs >= 4 => Some((vec![2, 3], 1, LengthKind::Llength)),
        "string" if nargs >= 2 => match texts[1].as_str() {
            "index" if nargs == 4 => Some((vec![3], 2, LengthKind::Strlen)),
            "range" if nargs == 5 => Some((vec![3, 4], 2, LengthKind::Strlen)),
            "replace" if nargs >= 5 => Some((vec![3, 4], 2, LengthKind::Strlen)),
            _ => None,
        },
        _ => None,
    }
}

/// Return the single `expr` argument of a `[expr <arg>]` command word, or
/// `None` for anything else. The word text is the verbatim `[…]`
/// substitution; the head is recognised via the registry's
/// structural expression descriptor, not a name match. A sole operand needs
/// no multi-argument concatenation capability.
fn expr_arg_from_command_word(
    word: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let inner = word.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?;
    let cmds = segment_commands_with_offset_and_config(inner, 0, config);
    let [cmd] = cmds.as_slice() else {
        return None;
    };
    if cmd.texts.len() != 2 {
        return None;
    }
    let head_is_expr = registry
        .and_then(|registry| registry.get_for_surface(&cmd.texts[0], registry.own_surface_query()))
        .is_some_and(|spec| spec.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Expr));
    if !head_is_expr {
        return None;
    }
    Some(cmd.texts[1].clone())
}

/// Parse `[llength $L] - N` / `[string length $s] - N` (with `N >= 1`)
/// into `(kind, container_word, end_offset)` where the Tcl end-offset is
/// `N - 1` (`[llength $L] - 1` → `end`). A bare `[llength $L]` (no
/// subtraction) is one past the last index and is rejected.
fn try_end_offset_from_length_expr(
    expr_text: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<(LengthKind, String, i64)> {
    let config = tcl_lexer::LexerConfig::for_profile(dialect);
    let node = parse_expr_for_profile(expr_text.trim(), dialect);
    let ExprNode::Binary {
        op: BinOp::Sub,
        left,
        right,
    } = node
    else {
        return None;
    };
    let ExprNode::Command { text: cmd_text, .. } = left.as_ref() else {
        return None;
    };
    let ExprNode::Literal { text: rhs, .. } = right.as_ref() else {
        return None;
    };
    let n = rhs.trim().parse::<i64>().ok()?;
    if n < 1 {
        return None;
    }
    if let Some(arg) = parse_length_arg(cmd_text, &["llength"], config) {
        return Some((LengthKind::Llength, arg, n - 1));
    }
    if let Some(arg) = parse_length_arg(cmd_text, &["string", "length"], config) {
        return Some((LengthKind::Strlen, arg, n - 1));
    }
    None
}

/// If `cmd_text` is `[<head…> <arg>]` (or the same without the brackets)
/// where the leading words equal `head`, return the trailing `<arg>` as
/// it segments (so the spelling matches the container word). Covers both
/// `llength $L` (one head word) and `string length $s` (two).
fn parse_length_arg(
    cmd_text: &str,
    head: &[&str],
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let inner = cmd_text
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(cmd_text)
        .trim();
    let cmds = segment_commands_with_offset_and_config(inner, 0, config);
    let [cmd] = cmds.as_slice() else {
        return None;
    };
    if cmd.texts.len() != head.len() + 1 {
        return None;
    }
    if cmd.texts[..head.len()]
        .iter()
        .zip(head)
        .any(|(t, h)| t != h)
    {
        return None;
    }
    cmd.texts.last().cloned()
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
