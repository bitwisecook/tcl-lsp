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

//! T100: tainted operands an expression or a command argument coerces to a number.

use super::super::{
    CommandRegistry, DiagCode, ExprNode, FxHashSet, HashMap, HashSet, MAX_EXPR_NODE_DEPTH,
    SinkCall, Span, Symbol, TaintLattice, TaintScan, TaintWarning, ValueKey, Version, WordExpr,
    dialect_to_point, scan_argument_names,
};
use super::classification::selected_sink_schema;
use super::sink_wrappers::var_consumed_by_sanitiser;

/// One `ExprNode::Var` occurrence sitting in a numeric-coercion-relevant
/// operator position — the KCS T100 "expr operand: numeric coercion"
/// hazard (see [`collect_coercion_operands`]).
pub(in super::super) struct CoercionOperand {
    pub(in super::super) name: String,
    /// Original parser node retains exact spelling and offsets for native read queries.
    pub(in super::super) variable: Option<ExprNode>,
    /// Offset of this occurrence relative to the expr's own source text,
    /// when known. `None` for a var recovered from an unparsed
    /// [`ExprNode::Raw`] fallback, which carries no per-occurrence
    /// position.
    pub(in super::super) rel_span: Option<(u32, u32)>,
}

/// `true` when a binary operator attempts numeric (or, for `And`/`Or`,
/// boolean) coercion of its operands — Tcl's arithmetic, bitwise, shift,
/// numeric-comparison, and logical operators all do. The pure string/list
/// operators (`eq`/`ne`/`lt`/`le`/`gt`/`ge`, `in`/`ni`, and the iRules
/// word-comparison forms `contains`/`starts_with`/`ends_with`/`equals`/
/// `matches_glob`/`matches_regex`) never coerce — `expr {$tainted eq
/// "danger"}` carries no numeric-coercion hazard at all, so flagging it
/// with that message would be a false positive.
const fn binop_coerces(op: crate::expr_ast::BinOp) -> bool {
    use crate::expr_ast::BinOp;
    !matches!(
        op,
        BinOp::StrEq
            | BinOp::StrNe
            | BinOp::StrLt
            | BinOp::StrLe
            | BinOp::StrGt
            | BinOp::StrGe
            | BinOp::In
            | BinOp::Ni
            | BinOp::Contains
            | BinOp::StartsWith
            | BinOp::EndsWith
            | BinOp::StrEquals
            | BinOp::Matches
            | BinOp::MatchesGlob
            | BinOp::MatchesRegex
    )
}

/// A complete reference under the current lexer configuration whose name
/// agrees with the expression's variable inventory.
pub(in super::super) fn holds_one_whole_var_ref(
    text: &str,
    names: &HashSet<String>,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Ok(Some(root)) = tcl_syntax::naming::variable_reference_root_bytes(text.as_bytes(), config)
    else {
        return false;
    };
    !root.contains(&0)
        && names.len() == 1
        && names
            .iter()
            .next()
            .is_some_and(|name| name.as_bytes() == root)
}

/// Collect every `Var` occurrence of `expr` that sits in a position where
/// Tcl attempts numeric or boolean coercion of the operand.
///
/// `in_context` is the coercion status of `expr` itself as seen by its
/// parent (the top-level call passes `true`: a braced expr's overall
/// result is used as a number/boolean by its caller). Recursion rules:
///
/// - `Binary`: both children inherit [`binop_coerces`] of the operator —
///   a string-safe operator (`eq`, `in`, …) clears the context for its
///   *own* operands, but a nested coercing sub-expression on either side
///   still flags independently (`($a + 1) eq "3"` still flags `$a`).
/// - `Unary`: all five unary operators (arithmetic negation, bitwise
///   complement, logical `!`/`not`) coerce their operand.
/// - `Ternary`: the condition is always boolean-coerced; each branch
///   *inherits* the ternary's own context, since whichever branch's value
///   flows out stands in for the ternary as a whole in the parent
///   operator (`($c ? $tainted : 0) + 1` must still flag `$tainted`).
/// - `Call` (a math function): every argument is numeric, regardless of
///   context.
/// - `Command` (`[cmd …]`, a nested command substitution): opaque — its
///   contents are a separate statement's own arguments, not an expr
///   operand at all, so this walk does not descend into it. If `cmd` is
///   itself a T100/T101/… sink, that nested statement's own sink
///   classification already covers it.
/// - `Raw` holding one whole variable reference (`$c` / `${c}`, per
///   [`holds_one_whole_var_ref`]): an operand, not unparsed source. It carries
///   no operator of its own, so — exactly like `Var`, which is what it denotes
///   — its coercion status is whatever its parent gives it, and it is flagged
///   only `in_context`. A flattened `switch` dispatch puts its subject in
///   this shape (`cfg_lower::switch_subject_operand`), under the `StrEq` that
///   compares it with an arm pattern; only the string modes (`-exact`,
///   `-glob`, `-regexp`) are flattened, so the `StrEq` context is the whole
///   answer for an arm. Tcl 9.1's numeric `-integer` mode stays a call, which
///   [`emit_numeric_coercion_warnings`] covers.
/// - `Raw` (any other unparsed fallback text): position-blind but
///   conservative — every variable found by [`ExprNode::vars`] is flagged
///   (`rel_span: None`) regardless of `in_context`, since the parser gave up
///   and a coercing operator may be hiding inside; erring towards a false
///   positive here is safer than silently dropping coverage.
pub(in super::super) fn collect_coercion_operands(
    expr: &ExprNode,
    in_context: bool,
    out: &mut Vec<CoercionOperand>,
    depth: u32,
    (config, grammar): (tcl_lexer::LexerConfig, tcl_dialect::LexerGrammar),
) {
    // Native-stack safety net: this walks the `ExprNode` tree,
    // one native frame per level. Past the cap, stop descending — a collector
    // that returns what it has gathered so far is the safe fallback (the only
    // effect is that operands buried deeper than the cap are not flagged;
    // never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match expr {
        ExprNode::Var {
            name, start, end, ..
        } => {
            if in_context {
                out.push(CoercionOperand {
                    name: name.clone(),
                    variable: Some(expr.clone()),
                    rel_span: Some((*start, end.saturating_add(1))),
                });
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Command { .. } => {}
        ExprNode::Binary { op, left, right } => {
            let child_ctx = binop_coerces(*op);
            collect_coercion_operands(left, child_ctx, out, depth + 1, (config, grammar));
            collect_coercion_operands(right, child_ctx, out, depth + 1, (config, grammar));
        }
        ExprNode::Unary { operand, .. } => {
            collect_coercion_operands(operand, true, out, depth + 1, (config, grammar));
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_coercion_operands(condition, true, out, depth + 1, (config, grammar));
            collect_coercion_operands(true_branch, in_context, out, depth + 1, (config, grammar));
            collect_coercion_operands(false_branch, in_context, out, depth + 1, (config, grammar));
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_coercion_operands(arg, true, out, depth + 1, (config, grammar));
            }
        }
        ExprNode::Raw { text } => {
            let names = (ExprNode::Raw { text: text.clone() }).vars_with_grammar(grammar);
            if !in_context && holds_one_whole_var_ref(text, &names, config) {
                return;
            }
            for name in names {
                out.push(CoercionOperand {
                    name,
                    variable: None,
                    rel_span: None,
                });
            }
        }
    }
}

pub(super) struct CoercionTaint {
    pub(in super::super) symbol: Symbol,
    pub(super) value: TaintLattice,
}

impl CoercionTaint {
    pub(in super::super) fn represented<S: std::hash::BuildHasher>(
        symbol: Symbol,
        version: Version,
        taints: &HashMap<ValueKey, TaintLattice, S>,
    ) -> Self {
        Self {
            symbol,
            value: taints
                .get(&(symbol, version))
                .copied()
                .unwrap_or(TaintLattice::clean()),
        }
    }
}

pub(super) fn expression_read_taint<S: std::hash::BuildHasher>(
    source: crate::ssa::SsaSourceView<'_>,
    node: &ExprNode,
    base: Option<u32>,
    registry: &CommandRegistry,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> Option<CoercionTaint> {
    let reference = source.read_expression_variable(node, base)?;
    if let Some(version) = reference.version {
        return Some(CoercionTaint::represented(
            reference.symbol,
            version,
            taints,
        ));
    }
    let contents = source.read_expression_variable_contents(node, base, registry)?;
    let mut value = if contents.unknown_residual {
        TaintLattice::tainted()
    } else {
        TaintLattice::clean()
    };
    if contents.includes_incoming {
        value = value.join(CoercionTaint::represented(reference.symbol, 0, taints).value);
    }
    for (block, index) in contents.writes {
        let statement = source
            .function()
            .blocks
            .get(&block)?
            .statements
            .get(index)?;
        value = value.join(
            statement
                .defs
                .get(&reference.symbol)
                .map_or_else(TaintLattice::tainted, |version| {
                    CoercionTaint::represented(reference.symbol, *version, taints).value
                }),
        );
    }
    Some(CoercionTaint {
        symbol: reference.symbol,
        value,
    })
}

/// Emit T100 warnings for numeric-coercion-risk operands of a braced
/// `expr` — an `AssignExpr`/`ExprEval` statement, or an `if`/`while`/`for`
/// branch condition (see [`emit_branch_condition_warnings`]).
///
/// `resolve` projects taint from the exact physical read and its represented
/// contents alternatives; a `None` result skips that
/// occurrence (no binding here). `base_offset`, when `Some`, is the
/// absolute source offset of `expr`'s own text — `ExprNode::Var` offsets
/// are relative to it, so a hit's absolute span is computable; `None`
/// (or a `Raw`-derived hit with no `rel_span`) falls back to
/// `fallback_span`, the caller's coarser span for the whole expr.
pub(super) fn emit_expr_coercion_warnings(
    expr: &ExprNode,
    base_offset: Option<u32>,
    fallback_span: Span,
    resolve: impl Fn(&CoercionOperand) -> Option<CoercionTaint>,
    warnings: &mut Vec<TaintWarning>,
    (config, grammar): (tcl_lexer::LexerConfig, tcl_dialect::LexerGrammar),
) {
    let mut hits = Vec::new();
    collect_coercion_operands(expr, true, &mut hits, 0, (config, grammar));
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    for hit in hits {
        let Some(read) = resolve(&hit) else {
            continue;
        };
        if read.value.is_conservative_global_seed_only()
            || !read.value.is_tainted()
            || !emitted.insert(read.symbol)
        {
            continue;
        }
        let span = match (hit.rel_span, base_offset) {
            (Some((s, e)), Some(base)) => Span::new(base + s, base + e),
            _ => fallback_span,
        };
        warnings.push(TaintWarning {
            source_sink: None,
            span,
            variable: hit.name.clone(),
            sink_command: "expr".to_owned(),
            code: DiagCode::T100,
            message: format!(
                "Tainted variable ${} flows into expr operand; \
                 numeric coercion may misinterpret value \
                 (use Tcl numeric-validation guards)",
                hit.name
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Emit T100 for a tainted variable in an argument word the command reads as
/// a number for this call — the registry's
/// [`tcl_registry::CommandSpec::taint_numeric_coercion`] (`switch -integer`'s
/// subject). The same hazard, message and carve-outs as a braced `expr`
/// operand: a brace-quoted mention is literal text, and a mention consumed by
/// a sanitiser substitution (`[string length $x]`) hands over a clean value.
pub(super) fn emit_numeric_coercion_warnings<S: std::hash::BuildHasher>(
    call: &SinkCall<'_>,
    env: &TaintScan<'_, S>,
    span: Span,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    warnings: &mut Vec<TaintWarning>,
) {
    let Some((spec, _command, _selected_args)) = selected_sink_schema(call) else {
        return;
    };
    let Some(shape) = spec.taint_numeric_coercion else {
        return;
    };
    let arg_refs: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let slots = shape.coerced_args(spec, &arg_refs, dialect_to_point(dialect));
    if slots.is_empty() {
        return;
    }
    let sink_label = format!("{} {} operand", spec.name, shape.option());
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    for slot in slots {
        let Some(arg) = call.args.get(slot) else {
            continue;
        };
        // Decoded argv text may contain a dollar sign that Tcl never read.
        // Keep the original word's substitution contract after decoding.
        let effective = call
            .tokens
            .and_then(crate::registry_invocation::effective_command_words);
        if call
            .effective
            .or(effective.as_ref())
            .and_then(|words| words.words.get(call.argument_offset + slot + 1))
            .is_some_and(|word| {
                matches!(
                    word,
                    WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                )
            })
        {
            continue;
        }
        let slot_call = SinkCall {
            args: &call.args[slot..=slot],
            argument_offset: call.argument_offset + slot,
            ..*call
        };
        for name in scan_argument_names(env, arg) {
            let Some(sym) = env.ssa.var_symbol(&name) else {
                continue;
            };
            if emitted.contains(&sym) || env.quoted_uses.is_some_and(|q| q.contains(&sym)) {
                continue;
            }
            let Some(&ver) = env.uses.get(&sym) else {
                continue;
            };
            let value = env
                .taints
                .get(&(sym, ver))
                .copied()
                .unwrap_or_else(TaintLattice::clean);
            if value.is_conservative_global_seed_only() {
                continue;
            }
            let tainted = value.is_tainted();
            if !tainted || var_consumed_by_sanitiser(&slot_call, &name) {
                continue;
            }
            let span = call
                .tokens
                .and_then(|tokens| tokens.argv.get(slot + 1).copied())
                .unwrap_or(span);
            warnings.push(TaintWarning {
                source_sink: None,
                span,
                message: format!(
                    "Tainted variable ${name} flows into {sink_label}; \
                     numeric coercion may misinterpret value \
                     (use Tcl numeric-validation guards)"
                ),
                variable: name,
                sink_command: sink_label.clone(),
                code: DiagCode::T100,
                replacement: None,
                fixes: Vec::new(),
            });
            emitted.insert(sym);
        }
    }
}
