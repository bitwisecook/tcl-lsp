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

//! Call-graph edges from command substitutions embedded in expressions, values and raw source text.

use super::call_facts::{scan_call_facts, strip_one_brace_layer};
use super::{LocalFacts, ScanCtx};
use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};

/// Walk an expression AST and record call-graph edges (and Body
/// recursion) for every `[cmd ...]` command substitution embedded
/// in the expression.
///
/// Without it, call-graph edges and unused-proc detection miss proc calls
/// embedded in control-flow predicates: the per-proc fact scanner walks
/// statement bodies but not expression operands.
/// `if {[q]} ...`, `while {[q]} ...`, and `for {init} {[q]} {next}
/// ...` left `q` unrecorded as a callee — flagging it as dead code
/// and missing the edge in `tcl callgraph`.
pub(super) fn scan_expr_for_calls(
    node: &crate::expr_ast::ExprNode,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    // Native-stack safety net: this walks the `ExprNode`
    // operator tree, one native frame per level. Past the cap, stop
    // descending — a fact collector that returns what it has recorded so far
    // is the safe fallback (call edges buried deeper than the cap are simply
    // not recorded; never a crash). Calls into `scan_source_for_calls` below
    // start that walker at bracket-text depth 0: a `[cmd …]` leaf's raw text
    // is a separate, independent recursion axis with its own cap.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } => {
            // Strip the outer `[…]` and segment the inner script.
            // Each top-level command becomes a `Statement::Call`
            // shape we can hand to `scan_call_facts` and recurse
            // into BODY-role args.
            let inner = text
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .unwrap_or(text.as_str());
            scan_source_for_calls(inner, ctx, facts, 0);
        }
        // Quoted strings may contain command substitutions (`"[q]"`), so
        // descend through the source text the same way as for `Command`;
        // a braced `{…}` one does not substitute.
        ExprNode::String { text, .. } => {
            if let Some(inner) = crate::word_subst::quoted_operand_body(text) {
                scan_source_for_calls(inner, ctx, facts, 0);
            }
        }
        ExprNode::Binary { left, right, .. } => {
            scan_expr_for_calls(left, ctx, facts, depth + 1);
            scan_expr_for_calls(right, ctx, facts, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            scan_expr_for_calls(operand, ctx, facts, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            scan_expr_for_calls(condition, ctx, facts, depth + 1);
            scan_expr_for_calls(true_branch, ctx, facts, depth + 1);
            scan_expr_for_calls(false_branch, ctx, facts, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                scan_expr_for_calls(a, ctx, facts, depth + 1);
            }
        }
        _ => {}
    }
}

/// Scan every `[cmd …]` command substitution embedded in a value /
/// expression `text` for call-graph edges. The text is lexed under the document
/// dialect; each [`TokenType::Cmd`] token's inner script is handed to
/// [`scan_source_for_calls`] (which resolves the head, applies the
/// callee's effects, and recurses into `BODY`-role args).
pub(super) fn scan_value_substitutions(
    text: &str,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    if ctx.positioned_substitutions {
        return;
    }
    let dialect = ctx.dialect;
    if !text.contains('[') {
        return;
    }
    let lexer = tcl_lexer::Lexer::with_source_map(
        tcl_lexer::SourceMap::new(text),
        tcl_lexer::LexerConfig::for_profile(dialect),
    );
    let Ok(tokens) = lexer.tokenise_all() else {
        return;
    };
    for tok in &tokens {
        if tok.kind != tcl_lexer::TokenType::Cmd {
            continue;
        }
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start <= end
            && end <= text.len()
            && text.is_char_boundary(start)
            && text.is_char_boundary(end)
        {
            // Thin dispatcher — carry the caller's bracket-text depth straight
            // through; `scan_source_for_calls` enforces the cap.
            scan_source_for_calls(&text[start..end], ctx, facts, depth);
        }
    }
}

pub(super) fn scan_source_for_calls(
    source: &str,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    let ctx = ScanCtx {
        current_tokens: None,
        parameter_body: None,
        ..ctx
    };
    facts.param_observations_incomplete = true;
    // Native-stack safety net: this recurses into `ArgRole::Body`
    // args, `apply` lambda bodies, and nested `[cmd …]` substitutions inside a
    // single word's raw text — a genuinely unbounded axis, independent of any
    // statement-tree cap (`catch {catch {catch {…}}}` / `apply {{} {apply {{}
    // {…}}}}` nested arbitrarily deep). Past the cap, stop descending: call
    // edges buried deeper than the cap are not recorded, never a crash.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let ScanCtx {
        dialect,
        identities,
        ..
    } = ctx;
    let surface = ctx.surface();
    // Scan the call graph under the
    // document dialect so `{*}` (8.4 / iRules) and `}{` (iRules) tokenise
    // the same way the rest of the analyser/lowering now does.
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_profile(dialect),
    );
    for cmd in commands {
        // Skip empty / non-literal command names — they're not
        // call-graph edges we can resolve at compile time.
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        // The head's effective identity, for every registry role query below.
        // `scan_call_facts` resolves it again for its own
        // queries — it is also reached from the `Statement::Call` arm, which
        // has no segmented command to hand it.
        let resolved: &str = identities.resolve_unpositioned(name).spec_name();
        let texts = cmd.args();
        scan_call_facts(name, texts, ctx, facts);
        // Recurse into BODY-role args (e.g. `catch {p}` → `{p}` is
        // BODY).  The registry resolves the role using the same
        // logic as the top-level scanner.
        let arg_strs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let body_indices = surface.arg_indices_for_role(
            resolved,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Body,
        );
        for idx in body_indices {
            if let Some(body_text) = texts.get(idx) {
                scan_source_for_calls(body_text, ctx, facts, depth + 1);
            }
        }
        // Recurse into an `ArgRole::LambdaLiteral` argument's real body
        // element (`apply {argList body ?ns?} …`) — the whole lambda literal
        // is a 2-element list, not a script, so scanning it as one (like a
        // plain `Body` arg) would misread the parameter word as a call-graph
        // edge to a non-existent proc and never reach the real body's own
        // calls at all.
        let lambda_indices = surface.arg_indices_for_role(
            resolved,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::LambdaLiteral,
        );
        for idx in lambda_indices {
            if let Some(&tok) = cmd.argv.get(idx + 1)
                && tok.kind == tcl_lexer::TokenType::Str
                && let Some(elems) = crate::lambda_literal::split_lambda_literal(source, tok)
                && let Some(body_span) = elems.body
                && let Some(body_text) =
                    source.get(body_span.start() as usize..body_span.end() as usize)
            {
                scan_source_for_calls(body_text, ctx, facts, depth + 1);
            }
        }
        // Recurse into EXPR-role args. `expr {…}` (and the registry's other
        // expression operands) re-parse and evaluate their argument, so a
        // `[cmd]` substitution inside it is a real call edge even when the
        // operand is brace-quoted — Tcl's `{…}` suppresses substitution at the
        // word level, but the expression engine re-evaluates it. Strip one
        // layer of braces (the common `expr {…}` form) and rescan the inner
        // text for command substitutions; an unbraced operand (`[q]`, `$x`)
        // falls through to the same scan unchanged. Without this, a recursive
        // call buried in `return [expr {[fib …]}]` is missed, leaving the call
        // graph incomplete (which under-converged the interproc taint fixpoint
        // and panicked the diagnostic worker on the debug guard).
        let expr_indices = surface.arg_indices_for_role(
            resolved,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Expr,
        );
        for idx in expr_indices {
            if let Some(arg) = texts.get(idx) {
                scan_value_substitutions(strip_one_brace_layer(arg), ctx, facts, depth + 1);
            }
        }
        // A `[cmd …]` substitution inside a *plain* value arg also executes in
        // this value/expression context (`set x [matchclass [HTTP::uri] …]`,
        // `if {[matchclass [HTTP::uri] …]}`), so its nested effects and edges
        // propagate too. This worker is reached
        // only from value/return/expr scanning. Positioned statement operands
        // use their retained WordExpr substitution carrier instead. Braced args are
        // inert (no `Cmd` token inside `{…}`); the
        // body/expr args handled above are idempotent under a re-scan.
        for arg in texts {
            if arg.contains('[') {
                scan_value_substitutions(arg, ctx, facts, depth + 1);
            }
        }
    }
}
