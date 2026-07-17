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

//! Expression-level shimmer detection (S100).
//!
//! `[expr]` arithmetic operators require numeric operands; string
//! comparison operators (`eq`, `ne`, `lt`, `le`, `gt`, `ge`) require
//! string values.  Using a value of the wrong type in an expression
//! forces a silent intrep conversion:
//!
//! - A `String` variable in an arithmetic binary op (`+`, `-`, `*`, …)
//!   → `String → Int/Double` shimmer.
//! - An `Int` or `Double` variable in a string comparison op (`eq`, `ne`,
//!   `lt`, `le`, `gt`, `ge`) → `Int/Double → String` shimmer.
//!
//! This pass walks every [`Statement::AssignExpr`] and
//! [`Statement::ExprEval`] in every SCCP-executable block and recurses
//! into the expression AST looking for such mismatches.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::TclType;

use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice};

use super::ShimmerWarning;
use super::graph::loop_body_blocks;
use super::hints::is_uncommitted_first_conversion;

/// Find expression-level shimmer warnings for a function.
///
/// Covers two expression sites:
/// 1. **`AssignExpr` / `ExprEval` statements** — `set x [expr {…}]` and
///    standalone `expr {…}`.
/// 2. **`Terminator::Branch` conditions** — the predicate of every
///    `if`/`while`/`for` construct.  Variable versions are resolved from
///    the block's `exit_versions` map (the versions live at the end of
///    the block, which is when the condition is evaluated).
#[must_use]
pub(crate) fn find_expr_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    values: &HashMap<ValueKey, LatticeValue>,
    registry: &tcl_registry::CommandRegistry,
    commit_facts: &super::commit::CommitFacts,
) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();
    let loop_blocks = loop_body_blocks(cfg);
    let commit_ctx = super::commit::CommitCtx {
        registry,
        ssa,
        types,
        values,
    };

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        // An expr-operator shimmer inside a loop body re-converts the operand
        // every iteration (S101); outside a loop it is one-time (S100).
        let in_loop = loop_blocks.contains(cfg.block_name(block_id));
        // Per-block de-duplication keyed on (statement span, variable): several
        // operands of the same statement that name the same variable emit one
        // warning, not one per operand.
        let mut seen: HashSet<(Span, String)> = HashSet::new();
        // The committed-intrep walker replays the commit transfer in step with
        // this walk, so each expr's operands see the state just before it.
        let mut commit_walker = commit_facts.walker(&commit_ctx, block_id);

        // 1. SSA statements: AssignExpr and ExprEval.
        for ss in &ssa_block.statements {
            match &ss.statement {
                Statement::AssignExpr {
                    expr,
                    span,
                    expr_base,
                    ..
                }
                | Statement::ExprEval {
                    expr,
                    span,
                    expr_base,
                    ..
                } => {
                    let mut ctx = ExprShimmerCtx {
                        uses: &ss.uses,
                        types,
                        values,
                        ssa,
                        commit: &commit_walker,
                        stmt_span: *span,
                        expr_base: *expr_base,
                        in_loop,
                        seen: &mut seen,
                        out: &mut out,
                    };
                    collect_expr_shimmers(&mut ctx, expr);
                }
                _ => {}
            }
            commit_walker.step(&ss.statement, &ss.uses);
        }

        // 2. Branch terminator condition (if/while/for predicate).
        if let Some(block) = cfg.blocks.get(&block_id)
            && let Some(Terminator::Branch {
                condition,
                span,
                condition_base,
                ..
            }) = &block.terminator
        {
            let branch_span = span.unwrap_or_else(|| Span::new(0, 0));
            // Use exit_versions: those are the variable versions in scope
            // when the condition is evaluated.
            let mut ctx = ExprShimmerCtx {
                uses: &ssa_block.exit_versions,
                types,
                values,
                ssa,
                commit: &commit_walker,
                stmt_span: branch_span,
                expr_base: *condition_base,
                in_loop,
                seen: &mut seen,
                out: &mut out,
            };
            collect_expr_shimmers(&mut ctx, condition);
        }
    }

    out
}

/// Read-only context + warning sinks threaded through one expr walk.
///
/// `uses` / `stmt_span` / `in_loop` are constant for a single
/// `collect_expr_shimmers` recursion (they describe the statement whose expr
/// is being walked); `seen` / `out` accumulate de-duplicated warnings.
struct ExprShimmerCtx<'a> {
    uses: &'a HashMap<Symbol, u32>,
    types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constant values, for the uncommitted-value ("pure string") check —
    /// a pure operand that is a valid instance of the required type converts for
    /// free, so it must not be flagged (see [`is_uncommitted_first_conversion`]).
    values: &'a HashMap<ValueKey, LatticeValue>,
    ssa: &'a SsaFunction,
    /// Committed-intrep state just before this statement ([`super::commit`]) —
    /// an operand whose value already committed a different intrep on every
    /// path genuinely re-represents here even when the lattice sees no
    /// mismatch.
    commit: &'a super::commit::CommitWalker<'a>,
    stmt_span: Span,
    /// Absolute source offset of the expression text's first byte, when it
    /// is a verbatim source slice (see [`crate::ir::IfClause::condition_base`]).
    /// Maps AST leaf offsets to absolute operand spans; `None` falls back to
    /// anchoring at `stmt_span`.
    expr_base: Option<u32>,
    in_loop: bool,
    seen: &'a mut HashSet<(Span, String)>,
    out: &'a mut Vec<ShimmerWarning>,
}

impl ExprShimmerCtx<'_> {
    /// The span a shimmer on `node` anchors to: the operand's own source
    /// range when the expression text is verbatim-anchored (the leaf's
    /// offsets shifted by `expr_base`), else the whole statement.  Leaf
    /// `end` offsets are *inclusive* (the expr lexer's convention), so the
    /// exclusive span end is `end + 1`.
    fn operand_span(&self, node: &ExprNode) -> Span {
        if let (Some(base), ExprNode::Var { start, end, .. }) = (self.expr_base, node)
            && end >= start
        {
            return Span::new(base + *start, base + *end + 1);
        }
        self.stmt_span
    }
}

fn collect_expr_shimmers(ctx: &mut ExprShimmerCtx<'_>, node: &ExprNode) {
    match node {
        ExprNode::Binary {
            op, left, right, ..
        } => {
            // Recurse into children first.
            collect_expr_shimmers(ctx, left);
            collect_expr_shimmers(ctx, right);

            match op {
                // Arithmetic, bitwise, and shift operators are an unconditional
                // numeric context with NO string fallback: Tcl reads each
                // operand with `Tcl_GetNumberFromObj`, which on success installs
                // the numeric intrep in place (clobbering String/List/Dict/
                // ByteArray) and on failure raises — so on any path that
                // executes without error, the shimmer really happened.
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor => {
                    check_numeric_operand(ctx, left, *op, NumericContext::Arithmetic);
                    check_numeric_operand(ctx, right, *op, NumericContext::Arithmetic);
                }

                // `&&`/`||` are a boolean context, not arithmetic
                // (`tclExecute.c` INST_LOR/INST_LAND use `TclGetBooleanFromObj`)
                // — but like arithmetic it has no string fallback: the operand's
                // intrep is replaced by a boolean/numeric one on success and the
                // command errors otherwise, so flagging a non-numeric intrep is
                // still sound. Only the wording and target type differ.
                BinOp::And | BinOp::Or => {
                    check_numeric_operand(ctx, left, *op, NumericContext::Boolean);
                    check_numeric_operand(ctx, right, *op, NumericContext::Boolean);
                }

                // `in`/`ni` convert the RIGHT operand to a list
                // (`TclListObjGetElements`) — a genuine intrep replacement for
                // a String/Dict/ByteArray value (tclsh-verified:
                // `expr {"b" in $L}` with a string-typed `$L` installs the
                // list intrep). The needle is read as a string (intrep kept).
                BinOp::In | BinOp::Ni => {
                    check_list_operand(ctx, right, *op);
                }

                // String comparison operators: operands should be String.
                // No rewrite to the numeric equivalent (eq→==, ne→!=, lt→<,
                // le→<=, gt→>, ge→>=) is ever offered — `eq`/`ne`/`lt`/…
                // always compare the operands' *string* representations,
                // never their numeric value, so the two families disagree
                // whenever the string forms don't sort/compare the same way
                // as the numbers they denote (`"10" lt "2"` is true
                // lexicographically but `10 < 2` is false; `"1.0" eq "1"` is
                // false but `1.0 == 1` is true). "Both operands are numeric"
                // does not make the rewrite safe, so only the informational
                // warning fires.
                BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe => {
                    check_string_operand(ctx, left, *op);
                    check_string_operand(ctx, right, *op);
                }

                // Everything else — notably the comparisons, both ordering
                // (`<`/`<=`/`>`/`>=`) and equality (`==`/`!=`) — is NOT
                // flagged. C Tcl probes each comparison operand with
                // `GetNumberFromObj` *without* generating a string rep and
                // falls back to a string comparison (both intreps kept) when
                // either operand is non-numeric; an operand's intrep is
                // replaced only when its OWN string happens to parse as a
                // number, which a static type cannot prove (verified on tclsh
                // 8.6: `expr {5 == $L}` with `$L` a list keeps the list
                // intrep; `expr {$s <= 5}` with `$s` = "hello" keeps
                // `string`). Flagging on "the sibling operand looks numeric"
                // was a verified false-positive class; the S102 flip-flop
                // detection still catches genuinely alternating intreps.
                _ => {}
            }
        }

        ExprNode::Unary { operand, .. } => {
            collect_expr_shimmers(ctx, operand);
        }

        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_shimmers(ctx, condition);
            collect_expr_shimmers(ctx, true_branch);
            collect_expr_shimmers(ctx, false_branch);
        }

        _ => {}
    }
}

/// Which no-string-fallback expr context a flagged operand sits in — decides
/// the diagnostic's wording and target type, not whether it fires.
#[derive(Clone, Copy)]
enum NumericContext {
    /// `+`/`-`/`*`/… — `Tcl_GetNumberFromObj`, intrep becomes Int/Double.
    Arithmetic,
    /// `&&`/`||` — `TclGetBooleanFromObj`, intrep becomes Boolean (or numeric).
    Boolean,
}

/// Emit a shimmer if `node` is a variable reference with a non-numeric
/// type used in a numeric/boolean coercion context.  The code is S101 inside
/// a loop body (per-iteration conversion) and S100 outside one.
fn check_numeric_operand(
    ctx: &mut ExprShimmerCtx<'_>,
    node: &ExprNode,
    op: BinOp,
    context: NumericContext,
) {
    let ExprNode::Var { text, .. } = node else {
        return;
    };
    let base = crate::naming::element_var_name(text);
    let Some(sym) = ctx.ssa.var_symbol(base) else {
        return;
    };
    let Some(&ver) = ctx.uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind() != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type() else {
        return;
    };
    let (to_type, context_name) = match context {
        NumericContext::Arithmetic => (TclType::Numeric, "arithmetic expression"),
        NumericContext::Boolean => (TclType::Boolean, "boolean context"),
    };
    // A prior use that committed a non-numeric intrep on every path makes this
    // operand a genuine second conversion (`set v 5; llength $v; expr {$v+1}`
    // re-represents List → Numeric) — even where the lattice type is numeric.
    let commit_state = ctx.commit.state_of(sym, ver);
    // Otherwise only flag clearly non-numeric types (String, List, Dict,
    // ByteArray). A byte array in an arithmetic context is a textbook C Tcl
    // shimmer: Tcl has no numeric intrep of its own, so `Tcl_GetNumberFromObj`
    // falls back to parsing the byte array's *string* rep (each byte
    // relabelled as a Latin-1 code point) as a number and, on success,
    // replaces the cached byte-array intrep with Int/Double — exactly the same
    // in-place intrep clobber as the String/List/Dict cases, just via the
    // byte-array route.
    let lattice_flags = matches!(
        current,
        TclType::String | TclType::List | TclType::Dict | TclType::ByteArray
    );
    if !commit_state.must_pay(to_type) {
        if !lattice_flags {
            return;
        }
        // A pure (uncommitted) operand whose value is a valid number / boolean
        // converts for free: `set s [string trim true]; expr {$s && 1}` and a
        // numeric-valued string in arithmetic are not shimmers. A committed
        // List/Dict/ByteArray, or a pure string that is not a valid instance
        // (`set s hello; expr {$s + 1}`), still fires.
        if is_uncommitted_first_conversion(current, to_type, ctx.values.get(&(sym, ver))) {
            return;
        }
    }
    let (from, from_label) = super::committed_from_label(&commit_state, current, to_type);
    // De-duplicate per (statement span, variable) within the block.
    if !ctx.seen.insert((ctx.stmt_span, base.to_owned())) {
        return;
    }
    let code = if ctx.in_loop {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    ctx.out.push(ShimmerWarning {
        span: ctx.operand_span(node),
        variable: base.to_owned(),
        from_type: from,
        to_type,
        command: format!("expr:{op:?}"),
        in_loop: ctx.in_loop,
        code,
        message: format!(
            "variable '{base}' has {from_label} intrep used in {context_name} \
             (operand of '{op}')",
        ),
        related: Vec::new(),
    });
}

/// Emit a shimmer for the RIGHT operand of `in`/`ni` when its type shows an
/// intrep that list conversion replaces (String/Dict/ByteArray → List). A
/// List-typed operand is already there; numeric intreps regenerate a string
/// then parse, which is the same replacement, but a single number is a
/// one-element list conversion so cheap it is not worth a warning.
fn check_list_operand(ctx: &mut ExprShimmerCtx<'_>, node: &ExprNode, op: BinOp) {
    let ExprNode::Var { text, .. } = node else {
        return;
    };
    let base = crate::naming::element_var_name(text);
    let Some(sym) = ctx.ssa.var_symbol(base) else {
        return;
    };
    let Some(&ver) = ctx.uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind() != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type() else {
        return;
    };
    // A prior use that committed a non-list intrep on every path makes this
    // membership test a genuine second conversion (`expr {$v+1}` then
    // `expr {"b" in $v}` re-represents Numeric → List).
    let commit_state = ctx.commit.state_of(sym, ver);
    let lattice_flags = matches!(
        current,
        TclType::String | TclType::Dict | TclType::ByteArray
    );
    if !commit_state.must_pay(TclType::List) {
        if !lattice_flags {
            return;
        }
        // A pure string is a free first conversion to a list — `set hay [string
        // trim "a b c"]; expr {"b" in $hay}` parses it once, losslessly (oracle:
        // the value goes pure → list). Only a committed Dict/ByteArray genuinely
        // re-represents on the `in` list conversion.
        if is_uncommitted_first_conversion(current, TclType::List, ctx.values.get(&(sym, ver))) {
            return;
        }
    }
    let (from, from_label) = super::committed_from_label(&commit_state, current, TclType::List);
    if !ctx.seen.insert((ctx.stmt_span, base.to_owned())) {
        return;
    }
    let code = if ctx.in_loop {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    ctx.out.push(ShimmerWarning {
        span: ctx.stmt_span,
        variable: base.to_owned(),
        from_type: from,
        to_type: TclType::List,
        command: format!("expr:{op:?}"),
        in_loop: ctx.in_loop,
        code,
        message: format!(
            "variable '{var}' has {from_label} intrep converted to a list by \
             '{op_word}' membership",
            var = base,
            op_word = if matches!(op, BinOp::In) { "in" } else { "ni" },
        ),
        related: Vec::new(),
    });
}

/// Emit a shimmer if `node` is a numeric variable used in a string
/// comparison.  S101 inside a loop body, S100 outside one.
///
/// No `CodeFix` is ever attached: `eq`/`ne`/`lt`/`le`/`gt`/`ge` compare the
/// operands' string representations, never their numeric value, so no
/// rewrite to the numeric-equivalent operator is generally safe — see the
/// `BinOp::StrEq | …` match arm's doc comment in [`collect_expr_shimmers`].
fn check_string_operand(ctx: &mut ExprShimmerCtx<'_>, node: &ExprNode, op: BinOp) {
    let ExprNode::Var { text, .. } = node else {
        return;
    };
    let base = crate::naming::element_var_name(text);
    let Some(sym) = ctx.ssa.var_symbol(base) else {
        return;
    };
    let Some(&ver) = ctx.uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind() != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type() else {
        return;
    };
    // A numeric variable in a string comparison does NOT lose its intrep —
    // `TclStringCmp` reads the string reps, which are generated once and
    // cached *alongside* the numeric intrep (dual-porting; tclsh-verified:
    // `set x 42; expr {$x eq "42"}` leaves `x` an int). So this is a
    // likely-intent hint (`eq` on numbers is usually a typo for `==`), not a
    // conversion cost: it stays S100 even inside a loop — the S101
    // "per-iteration cost" escalation would claim a cost that is paid at
    // most once — and the wording claims no representation change.
    if matches!(
        current,
        TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
    ) {
        // De-duplicate per (statement span, variable) within the block.
        if !ctx.seen.insert((ctx.stmt_span, base.to_owned())) {
            return;
        }
        let numeric_op = numeric_equivalent(op);
        ctx.out.push(ShimmerWarning {
            span: ctx.operand_span(node),
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::String,
            command: format!("expr:{op:?}"),
            in_loop: ctx.in_loop,
            code: DiagCode::S100,
            message: format!(
                "numeric variable '{base}' compared as a string ('{op}') — the \
                 numeric intrep is kept (a string rep is cached alongside); if \
                 a numeric comparison was intended, use '{numeric_op}' instead"
            ),
            related: Vec::new(),
        });
    }
}

/// The numeric-comparison equivalent of a string-comparison `BinOp`, used
/// only for the diagnostic's hint text — not a suggestion that the rewrite
/// is behaviourally equivalent. Panics on any other variant — only ever
/// called with `BinOp::Str{Eq,Ne,Lt,Le,Gt,Ge}` from
/// [`check_string_operand`]'s single call site.
fn numeric_equivalent(op: BinOp) -> &'static str {
    match op {
        BinOp::StrEq => "==",
        BinOp::StrNe => "!=",
        BinOp::StrLt => "<",
        BinOp::StrLe => "<=",
        BinOp::StrGt => ">",
        BinOp::StrGe => ">=",
        _ => unreachable!("numeric_equivalent called with a non-string-comparison BinOp"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Compute commit facts + run the expr detector for one function unit —
    /// the production wiring `find_shimmer_warnings` performs.
    fn expr_shimmers(
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Vec<ShimmerWarning> {
        let ctx = super::super::commit::CommitCtx {
            registry,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let facts = super::super::commit::compute_commit_facts(
            &fu.cfg,
            &ctx,
            &fu.sccp.executable_blocks,
            &fu.sccp.executable_edges,
        );
        find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &fu.sccp.values,
            registry,
            &facts,
        )
    }

    /// An Int variable in arithmetic — no shimmer.
    #[test]
    fn no_expr_shimmer_int_in_arithmetic() {
        let cu = CompilationUnit::build_for("set x 5\nset y [expr {$x + 1}]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        assert!(w.is_empty(), "unexpected expr shimmers: {w:?}");
    }

    /// A String variable used in arithmetic should produce S100.
    #[test]
    fn expr_shimmer_string_in_arithmetic() {
        let cu = CompilationUnit::build_for(
            "set x \"hello\"\nset y [expr {$x + 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::String);
        assert!(
            has_shimmer,
            "expected string-in-arithmetic shimmer, got: {w:?}"
        );
    }

    /// Comparison operators never flag: C Tcl probes each operand without
    /// generating a string rep and falls back to a string comparison — an
    /// operand's intrep is replaced only when its OWN string parses as a
    /// number, which the value (not the type) decides. tclsh-verified:
    /// `set s [string trim hello]; expr {$s == "5"}` and `expr {$s <= 5}`
    /// both leave `s` a `string`; `expr {5 == $L}` keeps `$L` a `list`.
    /// The old behaviour ("flag when the sibling operand looks numeric") was
    /// a verified false-positive class.
    #[test]
    fn expr_shimmer_comparisons_never_flag() {
        for src in [
            // String operand vs numeric literal — value "hello" cannot parse,
            // no shimmer happens at runtime (the old code flagged this).
            "set s [string trim hello]\nset y [expr {$s == \"5\"}]",
            "set s [string trim hello]\nset y [expr {$s <= 5}]",
            // Both string — string compare.
            "set s [string trim hello]\nset y [expr {$s == \"hello\"}]",
            "set s [string trim hello]\nset y [expr {$s < \"banana\"}]",
            // List operand — list intrep kept, string compare.
            "set lst [list a b c]\nset s [string trim hi]\nset y [expr {$lst > $s}]",
            "set lst [list a b c]\nset y [expr {5 == $lst}]",
        ] {
            let cu = CompilationUnit::build_for(src, &registry(), false);
            let fu = cu.function("::top").unwrap();
            let w = expr_shimmers(fu, &registry());
            assert!(
                !w.iter()
                    .any(|sw| sw.variable == "s" || sw.variable == "lst"),
                "comparison operands must not be flagged for {src:?}: {w:?}"
            );
        }
    }

    /// `&&`/`||` are a boolean coercion context with no string fallback: a
    /// String-typed operand genuinely loses its intrep (tclsh:
    /// `set s "true"; expr {$s && 1}` installs a boolean intrep). The wording
    /// says boolean, not arithmetic.
    #[test]
    fn expr_shimmer_boolean_context_flags_string_operand() {
        let cu = CompilationUnit::build_for(
            "set s [string trim true]\nset y [expr {$s && 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let warning = w.iter().find(|sw| sw.variable == "s");
        let warning = warning.expect("string operand of && must be flagged");
        assert_eq!(warning.to_type, TclType::Boolean);
        assert!(
            warning.message.contains("boolean context"),
            "wording must say boolean context: {}",
            warning.message
        );
    }

    /// `in`/`ni` convert the RIGHT operand to a list. A *committed* Dict
    /// haystack genuinely re-represents (Dict → List); a *pure* string haystack
    /// does not — `[string trim "a b c"]` is a pure string (oracle: it goes pure
    /// → list on first read), so its list conversion is free. A List-typed
    /// haystack is already a list, and the needle is read as a string — both
    /// silent.
    #[test]
    fn expr_shimmer_in_membership_flags_committed_not_pure_haystack() {
        // TP: a committed Dict haystack re-represents to a list.
        let cu = CompilationUnit::build_for(
            "set hay [dict create a 1 b 2]\nset y [expr {\"a\" in $hay}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let warning = w
            .iter()
            .find(|sw| sw.variable == "hay")
            .expect("committed Dict haystack of `in` must be flagged");
        assert_eq!(warning.from_type, TclType::Dict);
        assert_eq!(warning.to_type, TclType::List);

        // TN (issue #940): a pure string haystack is a free first conversion.
        let cu_pure = CompilationUnit::build_for(
            "set hay [string trim \"a b c\"]\nset y [expr {\"b\" in $hay}]",
            &registry(),
            false,
        );
        let fu_pure = cu_pure.function("::top").unwrap();
        let w_pure = expr_shimmers(fu_pure, &registry());
        assert!(
            !w_pure.iter().any(|sw| sw.variable == "hay"),
            "pure string haystack must not be flagged: {w_pure:?}"
        );

        // A List-typed haystack is already a list — silent.
        let cu2 = CompilationUnit::build_for(
            "set hay [list a b c]\nset y [expr {\"b\" in $hay}]",
            &registry(),
            false,
        );
        let fu2 = cu2.function("::top").unwrap();
        let w2 = expr_shimmers(fu2, &registry());
        assert!(
            !w2.iter().any(|sw| sw.variable == "hay"),
            "list haystack must not be flagged: {w2:?}"
        );

        // The needle is read as a string — a String-typed needle is silent.
        let cu3 = CompilationUnit::build_for(
            "set needle [string trim b]\nset hay [dict create a 1]\nset y [expr {$needle in $hay}]",
            &registry(),
            false,
        );
        let fu3 = cu3.function("::top").unwrap();
        let w3 = expr_shimmers(fu3, &registry());
        assert!(
            !w3.iter().any(|sw| sw.variable == "needle"),
            "the needle of `in` must not be flagged: {w3:?}"
        );
    }

    /// An Int variable used in a string comparison inside an `AssignExpr` produces S100.
    #[test]
    fn expr_shimmer_int_in_string_comparison_assign_expr() {
        let cu =
            CompilationUnit::build_for("set x 42\nset z [expr {$x eq \"42\"}]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::Int);
        assert!(
            has_shimmer,
            "expected Int-in-string-cmp shimmer, got: {w:?}"
        );
    }

    /// An Int variable used in a string comparison inside an `if` condition
    /// (`Terminator::Branch`) produces S100 via the branch-condition path.
    #[test]
    fn expr_shimmer_int_in_if_branch_condition() {
        let cu = CompilationUnit::build_for(
            "set x 42\nif {$x eq \"42\"} { set y 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::Int);
        assert!(
            has_shimmer,
            "expected Int-in-string-cmp shimmer in if-condition, got: {w:?}"
        );
    }

    /// The same variable used in several operands of one expression emits a
    /// single shimmer, not one per operand (per-block `(span, var)` dedup).
    #[test]
    fn expr_shimmer_dedups_repeated_operand() {
        let cu = CompilationUnit::build_for(
            "set x \"hi\"\nset y [expr {$x + $x + $x}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let xs: Vec<_> = w.iter().filter(|sw| sw.variable == "x").collect();
        assert_eq!(
            xs.len(),
            1,
            "repeated operand must emit one shimmer, got: {xs:?}"
        );
    }

    /// An arithmetic shimmer inside a loop body is a per-iteration cost
    /// (S101); the same shimmer outside a loop is one-time (S100).
    #[test]
    fn expr_shimmer_in_loop_is_s101() {
        let cu = CompilationUnit::build_for(
            "proc f {l} {\n  foreach x $l {\n    set y [expr {$x + 1}]\n  }\n}\n",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = expr_shimmers(fu, &registry());
        let s = w.iter().find(|sw| sw.variable == "x");
        assert!(s.is_some(), "expected expr shimmer for x in loop: {w:?}");
        let s = s.unwrap();
        assert_eq!(
            s.code,
            DiagCode::S101,
            "in-loop arithmetic shimmer must be S101"
        );
        assert!(s.in_loop);
    }

    /// A String literal compared with `eq` — no shimmer (String is correct type).
    #[test]
    fn no_expr_shimmer_string_in_string_comparison() {
        let cu = CompilationUnit::build_for(
            "set x \"hello\"\nif {$x eq \"world\"} { set y 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        // x is String; used with eq (string comparison) — no shimmer.
        let str_cmp_shimmers: Vec<_> = w
            .iter()
            .filter(|sw| sw.variable == "x" && sw.to_type == TclType::String)
            .collect();
        assert!(
            str_cmp_shimmers.is_empty(),
            "unexpected String-in-string-cmp shimmer: {str_cmp_shimmers:?}"
        );
    }

    /// A byte-array-typed variable used in arithmetic shimmers to Numeric —
    /// the same in-place intrep clobber as String/List/Dict, reached via
    /// `Tcl_GetNumberFromObj` falling back to the byte array's string rep.
    #[test]
    fn expr_shimmer_bytearray_in_arithmetic() {
        let cu = CompilationUnit::build_for(
            "set b [binary format c* {49 50}]\nset y [expr {$b + 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "b" && sw.from_type == TclType::ByteArray);
        assert!(
            has_shimmer,
            "expected bytearray-in-arithmetic shimmer, got: {w:?}"
        );
    }

    /// TP (range precision): the warning span is the offending operand
    /// itself (`$x` inside the braced expr), not the whole statement.
    #[test]
    fn expr_shimmer_span_narrows_to_operand_assign_expr() {
        let src = "set x \"hello\"\nset y [expr {$x + 1}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let s = w
            .iter()
            .find(|sw| sw.variable == "x")
            .unwrap_or_else(|| panic!("expected shimmer for x, got: {w:?}"));
        let expected = src.rfind("$x").unwrap();
        assert_eq!(
            (s.span.start() as usize, s.span.end() as usize),
            (expected, expected + 2),
            "span must cover exactly the `$x` operand, got {:?} in {src:?}",
            s.span
        );
    }

    /// TP (range precision): a branch-condition shimmer anchors at the
    /// operand inside the braced `if` condition.
    #[test]
    fn expr_shimmer_span_narrows_to_operand_in_branch_condition() {
        let src = "set x 42\nif {$x eq \"42\"} { set y 1 }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let s = w
            .iter()
            .find(|sw| sw.variable == "x")
            .unwrap_or_else(|| panic!("expected shimmer for x, got: {w:?}"));
        let expected = src.rfind("$x").unwrap();
        assert_eq!(
            (s.span.start() as usize, s.span.end() as usize),
            (expected, expected + 2),
            "span must cover exactly the `$x` operand, got {:?} in {src:?}",
            s.span
        );
    }

    /// TP (range precision): the narrowed span works inside a proc body too
    /// (offset-0 memo + rebase must not lose the expression anchor).
    #[test]
    fn expr_shimmer_span_narrows_to_operand_inside_proc() {
        let src = "proc f {} {\n  set x \"hello\"\n  set y [expr {$x * 2}]\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let w = expr_shimmers(fu, &registry());
        let s = w
            .iter()
            .find(|sw| sw.variable == "x")
            .unwrap_or_else(|| panic!("expected shimmer for x, got: {w:?}"));
        // The unit's spans are unit-relative; recover absolutes via abs_span.
        let abs = fu.abs_span(s.span);
        let expected = src.rfind("$x").unwrap();
        assert_eq!(
            (abs.start() as usize, abs.end() as usize),
            (expected, expected + 2),
            "span must cover exactly the `$x` operand, got {abs:?} in {src:?}"
        );
    }

    /// Fallback (no verbatim anchor): a *quoted* condition is reconstructed
    /// from multiple tokens, so no `condition_base` exists — the warning
    /// still fires, anchored at the condition span (which contains the
    /// operand), never a bogus narrowed range.
    #[test]
    fn expr_shimmer_span_falls_back_without_verbatim_anchor() {
        let src = "set x 42\nif \"$x eq 42\" { set y 1 }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        // The quoted form substitutes before parsing, so the shimmer may or
        // may not fire; when it does, the span must contain the whole
        // fallback range and be non-degenerate.
        for s in w.iter().filter(|sw| sw.variable == "x") {
            assert!(
                s.span.end() > s.span.start(),
                "fallback span must be non-degenerate: {s:?}"
            );
            let slice = &src[s.span.start() as usize..s.span.end() as usize];
            assert!(
                slice.contains("$x"),
                "fallback span must still contain the operand, got {slice:?}"
            );
        }
    }

    /// TN (dedup + narrowing): with the span narrowed to the operand, the
    /// per-statement dedup still emits one warning for `$x + $x`, anchored
    /// at the *first* occurrence.
    #[test]
    fn expr_shimmer_narrowed_span_dedups_to_first_operand() {
        let src = "set x \"hi\"\nset y [expr {$x + $x}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let xs: Vec<_> = w.iter().filter(|sw| sw.variable == "x").collect();
        assert_eq!(xs.len(), 1, "one warning per statement+var: {xs:?}");
        let first = src.find("{$x").unwrap() + 1;
        assert_eq!(
            (xs[0].span.start() as usize, xs[0].span.end() as usize),
            (first, first + 2),
            "dedup keeps the first operand's span"
        );
    }

    /// (Regression guard) `eq`/`ne`/`lt`/`le`/`gt`/`ge` never get an
    /// auto-fix, even when both operands are provably numeric — a prior
    /// version of this check offered a rewrite to the numeric-equivalent
    /// operator whenever both sides looked numeric, but Tcl's string
    /// comparison operators compare string *representations*, not numeric
    /// value: `"10" lt "2"` is true (lexicographic) while `10 < 2` is false,
    /// so the rewrite silently changes program behaviour. `ShimmerWarning`
    /// no longer carries a `fixes` field at all — this only re-confirms the
    /// informational warning still fires for the case that used to (wrongly)
    /// offer a fix.
    #[test]
    fn expr_shimmer_lt_both_numeric_still_fires_with_no_fix_offered() {
        let src = "set x 10\nset y 2\nset z [expr {$x lt $y}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = expr_shimmers(fu, &registry());
        let s = w
            .iter()
            .find(|sw| sw.variable == "x" || sw.variable == "y")
            .unwrap_or_else(|| panic!("expected an Int-in-string-cmp shimmer, got: {w:?}"));
        assert!(
            s.message.contains("if a numeric comparison was intended"),
            "expected the informational hint, got: {s:?}"
        );
    }
}
