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

//! Branch-folding optimiser pass.
//!
//! Two entry points, both folded into [`run`]:
//!
//! - `optimise_constant_branches`: for every
//!   [`ConstantBranch`] SCCP produced, emits an `O101`
//!   suggestion rewriting the condition to the literal boolean
//!   it folded to.
//! - `optimise_branch_proc_calls`: for every branch
//!   condition SCCP could *not* fold, tries propagation via
//!   [`substitute_expr_constants`] (from the
//!   [`super::helpers::expr_simplify`] toolkit).
//!   `propagate_into_branches` runs `substitute_expr_constants`
//!   first to build a working text, then probes the four AST
//!   rewriters in priority order — `strength_reduce` (`O113`)
//!   → `strlen` (`O117`) → `streq` (`O120`) → `instcombine`
//!   (`O110`).  The first rewriter that changes text wins its
//!   diagnostic code; **`O100`** ("Propagate constants into
//!   branch expression") only fires when substitution changed
//!   the text *and* none of the AST rewriters did.
//!
//! Switch-dispatch branches (the ones `cfg_builder` synthesises
//! inside `switch` blocks: a chain of `StrEq` probes against
//! `switch_next_*` fall-through blocks) are skipped — folding
//! them would produce misleading rewrites of user-visible source
//! text.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::expr_ast::{BinOp, ExprNode};
use crate::sccp::ConstantBranch;
use tcl_lexer::Span;

use super::helpers::expr_simplify::{
    OperandTypes, instcombine_expr_typed, operand_types, substitute_expr_constants,
    try_eq_ne_string_compare_simplify_expr, try_fold_expr, try_strength_reduce_expr_typed,
    try_strlen_simplify_expr, try_unwrap_expr_in_expr,
};
use super::propagation::sccp_constants_for;
use super::{Optimisation, PassContext};

/// Run the branch-folding pass — both
/// `optimise_constant_branches` and
/// `optimise_branch_proc_calls`.
///
/// Constant-branch pass (`optimise_constant_branches`):
///
/// - Code: `O101` ("Fold constant expression").
/// - Replacement: `"1"` or `"0"` depending on the folded value,
///   wrapped in braces when the source condition was braced.
/// - Span: the branch terminator's condition span.
/// - Switch-dispatch branches (`StrEq` condition + block or
///   targets whose names mention `switch_next`) are skipped.
///
/// Branch-proc-call propagation (`optimise_branch_proc_calls`):
///
/// - Code: `O100` ("Propagate constants into branch
///   expression") when SCCP could not fold the branch but the
///   constant-substitution pass produced a text change.
/// - Uses per-function SCCP lattice to build the constants map;
///   variables whose every tracked version agrees on a single
///   `Const` value participate.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    for fu in cu.analysable_functions() {
        fold_constant_branches(ctx, fu);
        propagate_into_branches(ctx, fu);
    }
}

/// Recover the whole braced condition word when the CFG `while` / `for`
/// loop-condition `Branch` span omits the closing brace.
///
/// The loop-condition lowering span can end one byte short of the closing
/// `}` (so a braced `while {1 < 2}` slices as `{1 < 2`). Left uncorrected,
/// the brace-balance check in the fold paths fails and a rewrite replaces
/// `{1 < 2` with `1`, leaving a stray `}` (`while 1}` — a malformation).
/// When the slice has an unmatched leading `{` and the byte just past it is
/// `}`, return the span extended by one so the braced word is whole;
/// otherwise return `span` unchanged. (Workaround for an off-by-one in the
/// loop-condition CFG span; the codegen path is unaffected.)
fn brace_corrected_span(source: &str, span: Span) -> Span {
    let range = span.as_range();
    if range.end >= source.len() {
        return span;
    }
    let text = &source[range.clone()];
    if text.starts_with('{')
        && !text.ends_with('}')
        && source.as_bytes().get(range.end) == Some(&b'}')
    {
        return Span::new(span.start(), span.end() + 1);
    }
    span
}

/// For every branch the SCCP pass *did not* fold, project the
/// per-function lattice into a constants map and try to rewrite
/// the condition text via [`substitute_expr_constants`]. Emits
/// `O100` on any text change.
fn propagate_into_branches(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    // Note: `constants` may be empty — the cascade (strength-reduce, streq,
    // instcombine) and the O115 redundant-`expr` unwrap still apply to a
    // branch condition with no propagable constants; this does not gate on a
    // non-empty constant map. Substitution is simply a no-op in that case.
    // No trace/alias filtering needed: `constants` is a pure projection of
    // `fu.sccp.values`, and `sccp()` itself already forces a traced or
    // frame-aliased variable's own lattice entry to `Overdefined` —
    // `run_load_forwarding` (O102) is the one pass that still needs its own
    // check, since it runs an independent def-use-chain scan that never
    // consults `fu.sccp` at all.
    let constants = sccp_constants_for(fu);
    // Numeric-type context so identity rewrites (`$x + 0` → `$x`, etc.) on a
    // branch condition fire only when the dropped operand is provably numeric.
    let numeric = operand_types(fu);
    let folded: HashSet<String> = fu
        .sccp
        .constant_branches
        .iter()
        .map(|cb| cb.block.clone())
        .collect();

    for (bn, block) in &fu.cfg.blocks {
        // `folded` holds block *names* (from each `ConstantBranch.block`);
        // resolve the iterated `BlockId` to its name to test membership.
        if folded.contains(fu.cfg.block_name(*bn)) {
            continue;
        }
        let Some(Terminator::Branch {
            condition,
            span: Some(span),
            ..
        }) = &block.terminator
        else {
            continue;
        };
        if is_switch_dispatch_cond(condition) {
            continue;
        }
        // Terminator span is relative to the unit's `base_offset`; absolutise
        // before slicing `ctx.source` / emitting.
        let span = fu.abs_span(*span);
        let span = brace_corrected_span(ctx.source, span);
        let range = span.as_range();
        if range.end > ctx.source.len() {
            continue;
        }
        let cond_text = &ctx.source[range];
        if cond_text.is_empty() {
            continue;
        }

        // Unwrap one level of braces so the substituter sees the
        // bare expression body; we re-wrap before reporting.
        let (inner, braced) = if let Some(body) = cond_text
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
        {
            (body, true)
        } else {
            (cond_text, false)
        };
        let rewrap = |text: &str| {
            if braced {
                format!("{{{text}}}")
            } else {
                text.to_owned()
            }
        };

        // O115: a branch condition that is itself a redundant `[expr {…}]`
        // wrapper (`if {[expr {$x}]} …`) unwraps to its inner expression.
        // Checked before the constant cascade. Unlike the rest of this
        // cascade — which folds the condition text using Tcl's own
        // internal expr evaluation (never a command dispatch, so
        // `rename`/`interp alias`-ing `expr` cannot affect it) — this
        // specific rewrite assumes the embedded `[expr {…}]` substring is
        // a genuine command substitution dispatching to builtin `expr`,
        // so it alone needs the trust gate.
        if let Some(unwrapped) = try_unwrap_expr_in_expr(inner)
            && unwrapped != inner
            && ctx.command_mutations.trusts("expr")
        {
            ctx.report(Optimisation::new(
                DiagCode::O115,
                "Remove redundant nested expr",
                span,
                rewrap(&unwrapped),
            ));
            continue;
        }

        // Cascade: substitute → strength-reduce → strlen → streq →
        // instcombine, with an O101 fold short-circuit.
        let sub = substitute_expr_constants(inner, &constants, ctx.dialect);
        let working = if sub.changed {
            sub.text.clone()
        } else {
            inner.to_owned()
        };

        // O101: if constant propagation makes the whole condition fold to a
        // literal (`if {$flag > 0}` with `flag == 1` → `1`), emit the fold in
        // preference to the partial-canonicalisation codes.
        if sub.changed
            && let Some(folded) = try_fold_expr(&working, ctx.dialect)
            && folded != inner
        {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                span,
                rewrap(&folded),
            ));
            continue;
        }

        let Some((code, message, final_text)) =
            branch_cascade(&working, sub.changed, &sub.text, &numeric)
        else {
            continue;
        };
        ctx.report(Optimisation::new(code, message, span, rewrap(&final_text)));
    }
}

/// The branch-condition simplification cascade: the first transform that
/// changes `working` wins its diagnostic code. Returns `None` when nothing
/// applies. `sub_changed`/`sub_text` carry the constant-substitution result
/// so a pure substitution (no further simplification) still emits O100.
fn branch_cascade(
    working: &str,
    sub_changed: bool,
    sub_text: &str,
    numeric: &OperandTypes,
) -> Option<(DiagCode, &'static str, String)> {
    let (sred, sred_changed) = try_strength_reduce_expr_typed(working, Some(numeric));
    if sred_changed {
        return Some((DiagCode::O113, "Strength-reduce expression", sred));
    }
    let (slen, slen_changed) = try_strlen_simplify_expr(working);
    if slen_changed {
        return Some((DiagCode::O117, "Simplify string length zero-check", slen));
    }
    let (streq, streq_changed) = try_eq_ne_string_compare_simplify_expr(working);
    if streq_changed {
        return Some((DiagCode::O120, "Use eq/ne for string comparison", streq));
    }
    let (combined, combined_changed) = instcombine_expr_typed(working, true, Some(numeric));
    if combined_changed {
        return Some((
            DiagCode::O110,
            "Canonicalise expression (InstCombine)",
            combined,
        ));
    }
    if sub_changed {
        return Some((
            DiagCode::O100,
            "Propagate constants into branch expression",
            sub_text.to_owned(),
        ));
    }
    None
}

fn is_switch_dispatch_cond(cond: &ExprNode) -> bool {
    matches!(
        cond,
        ExprNode::Binary {
            op: BinOp::StrEq,
            ..
        }
    )
}

fn fold_constant_branches(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    // No trace/alias filtering needed: `fu.sccp.constant_branches` is
    // derived entirely from `sccp()`'s own lattice `values`, and `sccp()`
    // itself already forces a traced or frame-aliased variable's lattice
    // entry to `Overdefined` — a branch reading one never resolves to a
    // constant decision in the first place, so it never lands in
    // `constant_branches` to begin with. O102 `run_load_forwarding` is the
    // one pass that still needs its own check, since it runs an independent
    // def-use-chain scan that never consults `fu.sccp` at all.
    for cb in &fu.sccp.constant_branches {
        let Some(block) = fu.cfg.block_by_name(&cb.block) else {
            continue;
        };
        let Some(Terminator::Branch {
            condition,
            span: Some(span),
            ..
        }) = &block.terminator
        else {
            continue;
        };

        if is_switch_dispatch(condition, cb) {
            continue;
        }

        // `span` is carried by `fu.cfg`'s terminator, so it is relative to the
        // unit's `base_offset` (0 for a real-position build, the body offset for
        // a memoised offset-0 unit); recover the absolute span before slicing
        // `ctx.source` or emitting.
        let span = fu.abs_span(*span);
        let span = brace_corrected_span(ctx.source, span);
        let source = ctx.source;
        let range = span.as_range();
        if range.end > source.len() {
            continue;
        }
        let cond_text = &source[range];
        if cond_text.is_empty() {
            continue;
        }

        let (prefix, suffix) = if cond_text.starts_with('{') && cond_text.ends_with('}') {
            ("{", "}")
        } else {
            ("", "")
        };

        let folded = if cb.value { "1" } else { "0" };
        let replacement = format!("{prefix}{folded}{suffix}");

        ctx.report(Optimisation::new(
            DiagCode::O101,
            "Fold constant expression",
            span,
            replacement,
        ));
    }
}

/// Return `true` when a constant-branch is the synthetic
/// `switch` dispatch chain `cfg_builder` emits. The dispatch is
/// always a `StrEq` test between the switch subject and a pattern
/// word, with a fall-through target block named `switch_next_*`.
fn is_switch_dispatch(cond: &ExprNode, cb: &ConstantBranch) -> bool {
    if !matches!(
        cond,
        ExprNode::Binary {
            op: BinOp::StrEq,
            ..
        }
    ) {
        return false;
    }
    if cb.block.contains("switch_next") {
        return true;
    }
    cb.taken_target.starts_with("switch_next") || cb.not_taken_target.starts_with("switch_next")
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use tcl_core_types::DiagCode;

    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    use crate::analyses::{ConstValue, LatticeValue};
    use crate::cfg::{Block, BlockId, Function as CfgFunction, Terminator};
    use crate::compilation_unit::{CompilationUnit, FunctionUnit};
    use crate::def_use::DefUseResult;
    use crate::expr_ast::{BinOp, ExprNode};
    use crate::interprocedural::InterproceduralAnalysis;
    use crate::sccp::{ConstantBranch, SccpResult};
    use crate::ssa::{SsaBlock, SsaFunction, Symbol};

    use super::super::{PassContext, PassId};
    use super::run;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn literal(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: u32::try_from(text.len()).unwrap_or(0),
        }
    }

    fn str_eq_cond(var: &str, lit: &str) -> ExprNode {
        ExprNode::Binary {
            op: BinOp::StrEq,
            left: Box::new(ExprNode::Var {
                text: format!("${var}"),
                name: var.into(),
                start: 0,
                end: 0,
            }),
            right: Box::new(ExprNode::Literal {
                text: lit.into(),
                start: 0,
                end: 0,
            }),
        }
    }

    fn empty_ssa_block(name: &str) -> SsaBlock {
        SsaBlock {
            name: name.into(),
            phis: Vec::new(),
            statements: Vec::new(),
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        }
    }

    /// Resolve an already-interned block name to its [`BlockId`].
    fn id_of(cfg: &CfgFunction, name: &str) -> BlockId {
        cfg.block_id(name).expect("block name interned")
    }

    /// Build an `SsaFunction` sharing `cfg`'s block-id numbering, with an
    /// empty `SsaBlock` inserted for every block in `cfg`.
    fn make_ssa(cfg: &CfgFunction) -> SsaFunction {
        let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        for (id, b) in &cfg.blocks {
            ssa.blocks.insert(*id, empty_ssa_block(&b.name));
        }
        ssa
    }

    /// The set of [`BlockId`]s for `names`, resolved against `cfg`.
    fn id_set(cfg: &CfgFunction, names: &[&str]) -> HashSet<BlockId> {
        names.iter().map(|n| id_of(cfg, n)).collect()
    }

    /// Build a synthetic [`FunctionUnit`] wrapping `cfg`, `ssa`,
    /// and a hand-rolled `SccpResult`. We skip the full pipeline
    /// so tests can feed the pass lattice values the SCCP driver
    /// would struggle to reach in isolation (mixed lattices,
    /// Overdefined, …).
    fn function_unit(
        name: &str,
        cfg: CfgFunction,
        ssa: SsaFunction,
        sccp: SccpResult,
    ) -> FunctionUnit {
        FunctionUnit {
            name: name.into(),
            cfg,
            ssa,
            def_use: DefUseResult::default(),
            sccp,
            types: std::collections::HashMap::new(),
            return_type: crate::types::TypeLattice::unknown(),
            taints: std::collections::HashMap::new(),
            rendered_props: std::collections::HashMap::new(),
            memory_ssa: None,
            complexity_guarded: false,
            base_offset: 0,
        }
    }

    /// Wrap a single [`FunctionUnit`] as a [`CompilationUnit`].
    fn compilation_unit(source: &str, fu: FunctionUnit) -> CompilationUnit {
        CompilationUnit {
            source: source.into(),
            ir_module: crate::ir::Module {
                source: String::new(),
                top_level: crate::ir::Script::new(),
                procedures: HashMap::new(),
                methods: HashMap::new(),
                body_units: HashMap::new(),
                redefined_procedures: std::collections::HashSet::new(),
                redefined_methods: std::collections::HashSet::new(),
                namespace_imports: Vec::new(),
                namespace_exports: Vec::new(),
                traced_commands: std::collections::BTreeSet::new(),
                has_dynamic_trace: false,
                traced_variables: std::collections::BTreeSet::new(),
                has_dynamic_variable_trace: false,
            },
            cfg_module: crate::cfg::CfgModule {
                top_level: fu.cfg.clone(),
                procedures: HashMap::new(),
            },
            top_level: fu,
            procedures: HashMap::new(),
            methods: HashMap::new(),
            body_units: HashMap::new(),
            interproc: None,
            connection_scope: None,
        }
    }

    fn branch_block(
        name: &str,
        cond: ExprNode,
        span: Span,
        true_t: BlockId,
        false_t: BlockId,
    ) -> Block {
        let mut b = Block::new(name);
        b.terminator = Some(Terminator::Branch {
            condition: cond,
            true_target: true_t,
            false_target: false_t,
            span: Some(span),
            condition_base: None,
        });
        b
    }

    fn ret_block(name: &str) -> Block {
        let mut b = Block::new(name);
        b.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        b
    }

    fn build_ctx(source: &str) -> PassContext<'_> {
        PassContext::new(source, InterproceduralAnalysis::default())
    }

    // Tests

    #[test]
    fn constant_true_condition_emits_o101_with_one() {
        let source = "if {1} { set x 1 } else { set y 2 }";
        let cond_span = Span::new(3, 6); // covers "{1}"
        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let t = cfg.intern_block("t");
        let e = cfg.intern_block("e");
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: literal("1"),
            true_target: t,
            false_target: e,
            span: Some(cond_span),
            condition_base: None,
        });
        cfg.blocks.insert(t, ret_block("t"));
        cfg.blocks.insert(e, ret_block("e"));

        let ssa = make_ssa(&cfg);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: id_set(&cfg, &["entry", "t"]),
            executable_edges: HashSet::default(),
            constant_branches: vec![ConstantBranch {
                block: "entry".into(),
                span: Some(cond_span),
                condition: "1".into(),
                value: true,
                taken_target: "t".into(),
                not_taken_target: "e".into(),
            }],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert_eq!(ctx.optimisations.len(), 1);
        let opt = &ctx.optimisations[0];
        assert_eq!(opt.code, DiagCode::O101);
        assert_eq!(opt.message, "Fold constant expression");
        assert_eq!(opt.replacement, "{1}");
        assert_eq!(opt.span, cond_span);
        assert!(opt.group.is_none());
        assert!(!opt.hint_only);
    }

    #[test]
    fn constant_false_condition_emits_o101_with_zero() {
        let source = "if {0} {a} else {b}";
        let cond_span = Span::new(3, 6);
        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let t = cfg.intern_block("t");
        let e = cfg.intern_block("e");
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: literal("0"),
            true_target: t,
            false_target: e,
            span: Some(cond_span),
            condition_base: None,
        });
        cfg.blocks.insert(t, ret_block("t"));
        cfg.blocks.insert(e, ret_block("e"));

        let ssa = make_ssa(&cfg);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: id_set(&cfg, &["entry", "e"]),
            executable_edges: HashSet::default(),
            constant_branches: vec![ConstantBranch {
                block: "entry".into(),
                span: Some(cond_span),
                condition: "0".into(),
                value: false,
                taken_target: "e".into(),
                not_taken_target: "t".into(),
            }],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert_eq!(ctx.optimisations.len(), 1);
        assert_eq!(ctx.optimisations[0].replacement, "{0}");
    }

    #[test]
    fn nested_constant_branches_fold_independently() {
        // entry branches on 1 → "mid"; mid branches on 0 → "inner_else".
        let source = "if {1} { if {0} {a} else {b} }";
        let outer_span = Span::new(3, 6); // "{1}"
        let inner_span = Span::new(12, 15); // "{0}"

        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let mid = cfg.intern_block("mid");
        let after = cfg.intern_block("after");
        let inner_then = cfg.intern_block("inner_then");
        let inner_else = cfg.intern_block("inner_else");
        cfg.blocks.insert(
            entry,
            branch_block("entry", literal("1"), outer_span, mid, after),
        );
        cfg.blocks.insert(
            mid,
            branch_block("mid", literal("0"), inner_span, inner_then, inner_else),
        );
        cfg.blocks.insert(inner_then, ret_block("inner_then"));
        cfg.blocks.insert(inner_else, ret_block("inner_else"));
        cfg.blocks.insert(after, ret_block("after"));

        let ssa = make_ssa(&cfg);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: id_set(&cfg, &["entry", "mid", "inner_else"]),
            executable_edges: HashSet::default(),
            constant_branches: vec![
                ConstantBranch {
                    block: "entry".into(),
                    span: Some(outer_span),
                    condition: "1".into(),
                    value: true,
                    taken_target: "mid".into(),
                    not_taken_target: "after".into(),
                },
                ConstantBranch {
                    block: "mid".into(),
                    span: Some(inner_span),
                    condition: "0".into(),
                    value: false,
                    taken_target: "inner_else".into(),
                    not_taken_target: "inner_then".into(),
                },
            ],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert_eq!(ctx.optimisations.len(), 2);
        let replacements: Vec<&str> = ctx
            .optimisations
            .iter()
            .map(|o| o.replacement.as_str())
            .collect();
        assert!(replacements.contains(&"{1}"));
        assert!(replacements.contains(&"{0}"));
    }

    #[test]
    fn mixed_lattice_with_one_branch_const_folds_only_that_branch() {
        // entry branches on Unknown → SCCP leaves it out of
        // constant_branches. A second block's branch folds. Only
        // the folded branch should yield an O101.
        let source = "if {$x} { if {1} {a} else {b} }";
        let folded_span = Span::new(13, 16); // "{1}"

        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let mid = cfg.intern_block("mid");
        let after = cfg.intern_block("after");
        let t = cfg.intern_block("t");
        let e = cfg.intern_block("e");
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: mid,
            false_target: after,
            span: Some(Span::new(3, 7)), // "{$x}"
            condition_base: None,
        });
        cfg.blocks
            .insert(mid, branch_block("mid", literal("1"), folded_span, t, e));
        cfg.blocks.insert(t, ret_block("t"));
        cfg.blocks.insert(e, ret_block("e"));
        cfg.blocks.insert(after, ret_block("after"));

        let mut ssa = make_ssa(&cfg);
        let x = ssa.intern_var("x");
        let mut values: HashMap<(Symbol, u32), LatticeValue> = HashMap::new();
        // Simulate a mixed / Overdefined lattice for x.
        values.insert(
            (x, 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(0), ConstValue::Int(1)]),
        );
        let sccp = SccpResult {
            values,
            executable_blocks: id_set(&cfg, &["entry", "mid", "t", "e"]),
            executable_edges: HashSet::default(),
            // Only the SCCP-proved constant branch appears here —
            // the ConstSet lattice does not produce one.
            constant_branches: vec![ConstantBranch {
                block: "mid".into(),
                span: Some(folded_span),
                condition: "1".into(),
                value: true,
                taken_target: "t".into(),
                not_taken_target: "e".into(),
            }],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert_eq!(ctx.optimisations.len(), 1);
        assert_eq!(ctx.optimisations[0].replacement, "{1}");
        // The folded one is in block "mid", span at offset 13.
        assert_eq!(ctx.optimisations[0].span, folded_span);
    }

    #[test]
    fn overdefined_lattice_produces_no_fold() {
        // Overdefined conditions never appear in
        // `constant_branches`, so the pass emits nothing.
        let source = "if {$x} { ok }";
        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let t = cfg.intern_block("t");
        let e = cfg.intern_block("e");
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: t,
            false_target: e,
            span: Some(Span::new(3, 7)),
            condition_base: None,
        });
        cfg.blocks.insert(t, ret_block("t"));
        cfg.blocks.insert(e, ret_block("e"));

        let mut ssa = make_ssa(&cfg);
        let x = ssa.intern_var("x");
        let mut values: HashMap<(Symbol, u32), LatticeValue> = HashMap::new();
        values.insert((x, 1), LatticeValue::Overdefined);
        let sccp = SccpResult {
            values,
            executable_blocks: id_set(&cfg, &["entry", "t", "e"]),
            executable_edges: HashSet::default(),
            constant_branches: Vec::new(),
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert!(ctx.optimisations.is_empty());
    }

    #[test]
    fn bare_condition_without_braces_yields_unwrapped_replacement() {
        // `if 1 { ... }` — the cond span covers the bare `1`.
        let source = "if 1 { a } else { b }";
        let cond_span = Span::new(3, 4);
        let mut cfg = CfgFunction::new("::top", "entry");
        let entry = cfg.entry;
        let t = cfg.intern_block("t");
        let e = cfg.intern_block("e");
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: literal("1"),
            true_target: t,
            false_target: e,
            span: Some(cond_span),
            condition_base: None,
        });
        cfg.blocks.insert(t, ret_block("t"));
        cfg.blocks.insert(e, ret_block("e"));

        let ssa = make_ssa(&cfg);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: id_set(&cfg, &["entry", "t"]),
            executable_edges: HashSet::default(),
            constant_branches: vec![ConstantBranch {
                block: "entry".into(),
                span: Some(cond_span),
                condition: "1".into(),
                value: true,
                taken_target: "t".into(),
                not_taken_target: "e".into(),
            }],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert_eq!(ctx.optimisations.len(), 1);
        assert_eq!(ctx.optimisations[0].replacement, "1");
    }

    #[test]
    fn switch_dispatch_branches_are_skipped() {
        // A `StrEq` condition where either the block name or a
        // target name mentions `switch_next` — the synthetic
        // dispatch chain — must not be folded.
        let source = "switch -- $s { a { one } b { two } }";
        let cond_span = Span::new(14, 17);
        let mut cfg = CfgFunction::new("::top", "switch_probe_0");
        let probe = cfg.entry;
        let arm_a = cfg.intern_block("arm_a");
        let switch_next_1 = cfg.intern_block("switch_next_1");
        cfg.blocks.insert(
            probe,
            branch_block(
                "switch_probe_0",
                str_eq_cond("s", "a"),
                cond_span,
                arm_a,
                switch_next_1,
            ),
        );
        cfg.blocks.insert(arm_a, ret_block("arm_a"));
        cfg.blocks.insert(switch_next_1, ret_block("switch_next_1"));

        let ssa = make_ssa(&cfg);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: id_set(&cfg, &["switch_probe_0", "arm_a"]),
            executable_edges: HashSet::default(),
            constant_branches: vec![ConstantBranch {
                block: "switch_probe_0".into(),
                span: Some(cond_span),
                condition: "$s eq a".into(),
                value: true,
                taken_target: "arm_a".into(),
                not_taken_target: "switch_next_1".into(),
            }],
        };
        let cu = compilation_unit(source, function_unit("::top", cfg, ssa, sccp));

        let mut ctx = build_ctx(&cu.source);
        run(&mut ctx, &cu);

        assert!(
            ctx.optimisations.is_empty(),
            "switch dispatch branches must not be folded: got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn branch_proc_calls_propagates_constant_into_non_folded_branch() {
        // `$cond` is Overdefined (written two different values
        // in branches of an outer if); the inner `if {$cond}` is
        // therefore not a SCCP constant branch. The branch-
        // proc-calls pass should attempt substitution and — since
        // $cond is Overdefined — emit nothing. This test exercises
        // the "no constant available" code path.
        let cu = CompilationUnit::build_for(
            "set cond 1\nif {$flag} { set cond 2 }\nif {$cond} { set x 1 }",
            &registry(),
            false,
        );
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        // No O100 because $cond is not a single constant across
        // all tracked versions.
        assert!(
            !ctx.optimisations.iter().any(|o| o.code == DiagCode::O100),
            "unexpected O100 for Overdefined variable: {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn branch_proc_calls_emits_o100_when_const_propagates() {
        // `set x 5` makes $x constant; `if {$x < $y}` has one
        // tracked version of $x, which SCCP collapses. The
        // branch condition `$x < $y` is not a SCCP constant
        // branch (because $y is Unknown), so substituting $x → 5
        // is the only rewrite. Expect an O100.
        let cu =
            CompilationUnit::build_for("set x 5\nif {$x < $y} { set z 1 }", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        let got = ctx
            .optimisations
            .iter()
            .find(|o| o.code == DiagCode::O100 && o.message.contains("branch"));
        assert!(
            got.is_some(),
            "expected O100 propagating x=5 into branch, got {:?}",
            ctx.optimisations,
        );
        let opt = got.unwrap();
        assert!(
            opt.replacement.contains('5'),
            "replacement should contain the propagated value, got {:?}",
            opt.replacement,
        );
    }

    #[test]
    fn branch_folding_runs_via_run_passes_dispatch() {
        // Smoke-test the PassId::BranchFolding wiring end-to-end
        // from a real source string using the full pipeline.
        let cu =
            CompilationUnit::build_for("if {1} { set x 1 } else { set y 2 }", &registry(), false);
        let mut ctx = build_ctx(&cu.source);
        super::super::run_passes(&mut ctx, &cu, &[PassId::BranchFolding]);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement.contains('1')),
            "expected an O101 fold via run_passes, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn branch_condition_redundant_expr_emits_o115() {
        // `if {[expr {$x}]} …` — the condition is a redundant `[expr {…}]`
        // wrapper; it unwraps to `$x`. Exercised at top level (proc bodies
        // with a returning branch collapse to a straight CFG).
        let cu = CompilationUnit::build_for(
            "if {[expr {$x}]} { puts a } else { puts b }",
            &registry(),
            false,
        );
        let mut ctx = build_ctx(&cu.source);
        super::super::run_passes(&mut ctx, &cu, &[PassId::BranchFolding]);
        let o115 = ctx
            .optimisations
            .iter()
            .find(|o| o.code == DiagCode::O115)
            .expect("expected an O115 unwrap on the branch condition");
        assert_eq!(o115.replacement, "{$x}");
    }

    #[test]
    fn renamed_expr_blocks_branch_condition_unwrap() {
        // FP guard: `if {[expr {$x}]} …` embeds a genuine command
        // substitution — once `expr` is renamed anywhere in the module,
        // `[expr {$x}]` no longer means "evaluate $x as an expression", so
        // the O115 unwrap must not fire.
        let source = "rename expr real_expr\nif {[expr {$x}]} { puts a } else { puts b }";
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = build_ctx(&cu.source);
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
        super::super::run_passes(&mut ctx, &cu, &[PassId::BranchFolding]);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O115),
            "must not unwrap through a renamed expr: {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn plain_branch_condition_fold_unaffected_by_expr_rename() {
        // TN/control: `if`/`while` conditions are evaluated by Tcl's own
        // internal expr engine, never by dispatching a command literally
        // named "expr" — so renaming `expr` elsewhere in the module must
        // NOT block a plain (no explicit `[expr {…}]` substring) constant
        // branch fold.
        let source = "rename expr real_expr\nif {2 + 2 == 4} { puts a } else { puts b }";
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = build_ctx(&cu.source);
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
        super::super::run_passes(&mut ctx, &cu, &PassId::all());
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O101 || o.code == DiagCode::O112),
            "plain condition fold must be unaffected by an unrelated expr rename: {:?}",
            ctx.optimisations,
        );
    }
}
