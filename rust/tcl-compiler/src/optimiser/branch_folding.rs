//! Branch-folding optimiser pass (C30a).
//!
//! Ported from `core/compiler/optimiser/_branch_folding.py`. For
//! every [`ConstantBranch`] that SCCP produced while analysing a
//! function, emit an `O101` suggestion to replace the condition
//! with the literal boolean the branch folded to.
//!
//! The Python pass has a second entry point —
//! `optimise_branch_proc_calls` — that re-runs expression
//! simplification / constant-propagation on branch conditions
//! SCCP could not resolve. That entry point depends on the
//! `expr_simplify` and `propagation` passes, which are still
//! deferred; it will land alongside those follow-ups.
//!
//! Switch-dispatch branches (the ones `cfg_builder` synthesises
//! inside `switch` blocks: a chain of `StrEq` probes against
//! `switch_next_*` fall-through blocks) are skipped — folding
//! them would produce misleading rewrites of user-visible source
//! text.

use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::expr_ast::{BinOp, ExprNode};
use crate::sccp::ConstantBranch;

use super::{Optimisation, PassContext};

/// Run the branch-folding pass.
///
/// Appends one [`Optimisation`] per foldable constant branch
/// across every function in `cu`. Matches the Python
/// `optimise_constant_branches` behaviour:
///
/// - Code: `O101` ("Fold constant expression").
/// - Replacement: `"1"` or `"0"` depending on the folded value,
///   wrapped in braces when the source condition was braced.
/// - Span: the branch terminator's condition span.
/// - Switch-dispatch branches (`StrEq` condition + block or
///   targets whose names mention `switch_next`) are skipped.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    for fu in cu.functions() {
        fold_constant_branches(ctx, fu);
    }
}

fn fold_constant_branches(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    for cb in &fu.sccp.constant_branches {
        let Some(block) = fu.cfg.blocks.get(&cb.block) else {
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
            "O101",
            "Fold constant expression",
            *span,
            replacement,
        ));
    }
}

/// Return `true` when a constant-branch is the synthetic
/// `switch` dispatch chain `cfg_builder` emits. The dispatch is
/// always a `StrEq` test between the switch subject and a pattern
/// word, with a fall-through target block named `switch_next_*`.
fn is_switch_dispatch(cond: &ExprNode, cb: &ConstantBranch) -> bool {
    if !matches!(cond, ExprNode::Binary { op: BinOp::StrEq, .. }) {
        return false;
    }
    if cb.block.contains("switch_next") {
        return true;
    }
    cb.taken_target.starts_with("switch_next")
        || cb.not_taken_target.starts_with("switch_next")
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    use crate::analyses::{ConstValue, LatticeValue};
    use crate::cfg::{Block, Function as CfgFunction, Terminator};
    use crate::compilation_unit::{CompilationUnit, FunctionUnit};
    use crate::def_use::DefUseResult;
    use crate::expr_ast::{BinOp, ExprNode};
    use crate::interprocedural::InterproceduralAnalysis;
    use crate::sccp::{ConstantBranch, SccpResult};
    use crate::ssa::{SsaBlock, SsaFunction};

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

    fn make_ssa(blocks: &[&str]) -> SsaFunction {
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: blocks[0].into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        for b in blocks {
            ssa.blocks.insert((*b).into(), empty_ssa_block(b));
        }
        ssa
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
            memory_ssa: None,
        }
    }

    /// Wrap a single [`FunctionUnit`] as a [`CompilationUnit`].
    fn compilation_unit(source: &str, fu: FunctionUnit) -> CompilationUnit {
        CompilationUnit {
            source: source.into(),
            ir_module: crate::ir::Module {
                top_level: crate::ir::Script::new(),
                procedures: HashMap::new(),
                methods: HashMap::new(),
                redefined_procedures: std::collections::HashSet::new(),
            },
            cfg_module: crate::cfg::CfgModule {
                top_level: fu.cfg.clone(),
                procedures: HashMap::new(),
            },
            top_level: fu,
            procedures: HashMap::new(),
            interproc: None,
        }
    }

    fn branch_block(
        name: &str,
        cond: ExprNode,
        span: Span,
        true_t: &str,
        false_t: &str,
    ) -> Block {
        let mut b = Block::new(name);
        b.terminator = Some(Terminator::Branch {
            condition: cond,
            true_target: true_t.into(),
            false_target: false_t.into(),
            span: Some(span),
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

    // -- Tests --------------------------------------------------------------

    #[test]
    fn constant_true_condition_emits_o101_with_one() {
        let source = "if {1} { set x 1 } else { set y 2 }";
        let cond_span = Span::new(3, 6); // covers "{1}"
        let mut cfg = CfgFunction::new("::top", "entry");
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Branch {
            condition: literal("1"),
            true_target: "t".into(),
            false_target: "e".into(),
            span: Some(cond_span),
        });
        cfg.blocks.insert("t".into(), ret_block("t"));
        cfg.blocks.insert("e".into(), ret_block("e"));

        let ssa = make_ssa(&["entry", "t", "e"]);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: ["entry", "t"].iter().map(|s| (*s).into()).collect(),
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
        assert_eq!(opt.code, "O101");
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
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Branch {
            condition: literal("0"),
            true_target: "t".into(),
            false_target: "e".into(),
            span: Some(cond_span),
        });
        cfg.blocks.insert("t".into(), ret_block("t"));
        cfg.blocks.insert("e".into(), ret_block("e"));

        let ssa = make_ssa(&["entry", "t", "e"]);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: ["entry", "e"].iter().map(|s| (*s).into()).collect(),
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
        cfg.blocks.insert(
            "entry".into(),
            branch_block("entry", literal("1"), outer_span, "mid", "after"),
        );
        cfg.blocks.insert(
            "mid".into(),
            branch_block("mid", literal("0"), inner_span, "inner_then", "inner_else"),
        );
        cfg.blocks.insert("inner_then".into(), ret_block("inner_then"));
        cfg.blocks.insert("inner_else".into(), ret_block("inner_else"));
        cfg.blocks.insert("after".into(), ret_block("after"));

        let ssa = make_ssa(&["entry", "mid", "inner_then", "inner_else", "after"]);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: ["entry", "mid", "inner_else"]
                .iter()
                .map(|s| (*s).into())
                .collect(),
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
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: "mid".into(),
            false_target: "after".into(),
            span: Some(Span::new(3, 7)), // "{$x}"
        });
        cfg.blocks.insert(
            "mid".into(),
            branch_block("mid", literal("1"), folded_span, "t", "e"),
        );
        cfg.blocks.insert("t".into(), ret_block("t"));
        cfg.blocks.insert("e".into(), ret_block("e"));
        cfg.blocks.insert("after".into(), ret_block("after"));

        let ssa = make_ssa(&["entry", "mid", "t", "e", "after"]);
        let mut values: HashMap<(String, u32), LatticeValue> = HashMap::new();
        // Simulate a mixed / Overdefined lattice for x.
        values.insert(
            ("x".into(), 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(0), ConstValue::Int(1)]),
        );
        let sccp = SccpResult {
            values,
            executable_blocks: ["entry", "mid", "t", "e"]
                .iter()
                .map(|s| (*s).into())
                .collect(),
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
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: "t".into(),
            false_target: "e".into(),
            span: Some(Span::new(3, 7)),
        });
        cfg.blocks.insert("t".into(), ret_block("t"));
        cfg.blocks.insert("e".into(), ret_block("e"));

        let ssa = make_ssa(&["entry", "t", "e"]);
        let mut values: HashMap<(String, u32), LatticeValue> = HashMap::new();
        values.insert(("x".into(), 1), LatticeValue::Overdefined);
        let sccp = SccpResult {
            values,
            executable_blocks: ["entry", "t", "e"].iter().map(|s| (*s).into()).collect(),
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
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Branch {
            condition: literal("1"),
            true_target: "t".into(),
            false_target: "e".into(),
            span: Some(cond_span),
        });
        cfg.blocks.insert("t".into(), ret_block("t"));
        cfg.blocks.insert("e".into(), ret_block("e"));

        let ssa = make_ssa(&["entry", "t", "e"]);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: ["entry", "t"].iter().map(|s| (*s).into()).collect(),
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
        cfg.blocks.insert(
            "switch_probe_0".into(),
            branch_block(
                "switch_probe_0",
                str_eq_cond("s", "a"),
                cond_span,
                "arm_a",
                "switch_next_1",
            ),
        );
        cfg.blocks.insert("arm_a".into(), ret_block("arm_a"));
        cfg.blocks.insert("switch_next_1".into(), ret_block("switch_next_1"));

        let ssa = make_ssa(&["switch_probe_0", "arm_a", "switch_next_1"]);
        let sccp = SccpResult {
            values: HashMap::new(),
            executable_blocks: ["switch_probe_0", "arm_a"]
                .iter()
                .map(|s| (*s).into())
                .collect(),
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
    fn branch_folding_runs_via_run_passes_dispatch() {
        // Smoke-test the PassId::BranchFolding wiring end-to-end
        // from a real source string using the full pipeline.
        let cu = CompilationUnit::build_for(
            "if {1} { set x 1 } else { set y 2 }",
            &registry(),
            false,
        );
        let mut ctx = build_ctx(&cu.source);
        super::super::run_passes(&mut ctx, &cu, &[PassId::BranchFolding]);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == "O101" && o.replacement.contains('1')),
            "expected an O101 fold via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
