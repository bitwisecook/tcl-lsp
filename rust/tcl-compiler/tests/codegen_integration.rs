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

//! Integration tests for the codegen emitter pipeline.
//!
//! Builds small CFG fixtures and runs them through `codegen_function`
//! / `codegen_module` to verify the resulting `FunctionAsm` shape.

use std::collections::{HashMap, HashSet};

use tcl_compiler::cfg::{Block, BlockId, CfgModule, Function as CfgFunction, Terminator};
use tcl_compiler::codegen::{Op, codegen_function, codegen_module};
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::{Module as IrModule, Script, Statement};
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

fn sp() -> Span {
    Span::new(0, 0)
}

fn toplevel_with(statements: Vec<Statement>) -> CfgFunction {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    {
        let blk = cfg.blocks.get_mut(&entry).unwrap();
        blk.statements = statements;
        blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    cfg
}

fn proc_with(name: &str, params: &[&str], statements: Vec<Statement>) -> CfgFunction {
    let mut cfg = CfgFunction::new(name, "entry_0");
    let entry = cfg.entry;
    {
        let blk = cfg.blocks.get_mut(&entry).unwrap();
        blk.statements = statements;
        blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    let _ = params;
    cfg
}

// Top-level scripts

#[test]
fn empty_toplevel_has_done_or_return_imm() {
    let cfg = toplevel_with(vec![]);
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &[], false, &registry);
    assert_eq!(asm.name, "::top");
    let last = asm.instructions.last().unwrap().op;
    assert!(matches!(last, Op::DONE | Op::RETURN_IMM));
}

#[test]
fn set_const_toplevel() {
    let cfg = toplevel_with(vec![Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        name_braced: false,
        value: "42".into(),
        value_span: None,
    }]);
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::PUSH1));
    assert!(ops.contains(&Op::STORE_STK));
}

#[test]
fn multiple_statements_numbered() {
    let cfg = toplevel_with(vec![
        Statement::AssignConst {
            span: sp(),
            name: "a".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::AssignConst {
            span: sp(),
            name: "b".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        },
    ]);
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &[], false, &registry);
    // Second statement should get a startCommand (or generic peephole path)
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::STORE_STK));
    // Output ends cleanly
    let last = asm.instructions.last().unwrap().op;
    assert!(matches!(last, Op::DONE | Op::RETURN_IMM));
}

#[test]
fn call_generates_invoke() {
    let cfg = toplevel_with(vec![Statement::Call {
        span: sp(),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["hello".into()],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }]);
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::INVOKE_STK1));
}

// Proc bodies

#[test]
fn empty_proc_pushes_empty_and_dones() {
    let cfg = proc_with("::foo", &[], vec![]);
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &[], true, &registry);
    // The peephole pass collapses the trailing pop into done, so we
    // should end with DONE.
    assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn proc_return_param_loads_and_dones() {
    let mut cfg = CfgFunction::new("::f", "entry_0");
    let entry = cfg.entry;
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        value: Some("${x}".into()),
        span: None,
        expr: None,
        braced: false,
    });
    let registry = CommandRegistry::build_default();
    let asm = codegen_function(&cfg, &["x"], true, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::LOAD_SCALAR1));
    assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
    // The LVT should have x at slot 0
    assert_eq!(asm.lvt.entries()[0], "x");
}

// If / branching

#[test]
fn if_else_diamond_emits_conditional_jump() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let then = cfg.intern_block("if_then_1");
    cfg.blocks.insert(then, Block::new("if_then_1"));
    let els = cfg.intern_block("if_else_1");
    cfg.blocks.insert(els, Block::new("if_else_1"));
    let end = cfg.intern_block("if_end_1");
    cfg.blocks.insert(end, Block::new("if_end_1"));

    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        },
        true_target: then,
        false_target: els,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks
        .get_mut(&els)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    // Should include a conditional jump (possibly shrunk)
    assert!(
        ops.contains(&Op::JUMP_FALSE4) || ops.contains(&Op::JUMP_FALSE1),
        "expected conditional jump, got {ops:?}"
    );
}

#[test]
fn if_const_true_dead_branch_eliminated() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let then = cfg.intern_block("if_then_1");
    cfg.blocks.insert(then, Block::new("if_then_1"));
    let els = cfg.intern_block("if_else_1");
    cfg.blocks.insert(els, Block::new("if_else_1"));
    let end = cfg.intern_block("if_end_1");
    cfg.blocks.insert(end, Block::new("if_end_1"));

    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: then,
        false_target: els,
        span: None,
        condition_base: None,
    });
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    // No conditional jump for a constant-true branch
    assert!(
        !ops.contains(&Op::JUMP_FALSE4) && !ops.contains(&Op::JUMP_FALSE1),
        "expected no conditional jump, got {ops:?}"
    );
}

#[test]
fn switch_dispatch_emits_jump_table() {
    use tcl_compiler::expr_ast::BinOp;

    // `end: pat.len() as u32` — the fixture patterns are short string literals
    // nowhere near `u32::MAX`, so the source-range cast never truncates.
    #[allow(clippy::cast_possible_truncation)]
    fn str_eq_branch(var: &str, pat: &str, tt: BlockId, ft: BlockId) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Binary {
                op: BinOp::StrEq,
                left: Box::new(ExprNode::Var {
                    text: format!("${var}"),
                    name: var.into(),
                    start: 0,
                    end: 0,
                }),
                right: Box::new(ExprNode::Literal {
                    text: pat.into(),
                    start: 0,
                    end: pat.len() as u32,
                }),
            },
            true_target: tt,
            false_target: ft,
            span: None,
            condition_base: None,
        }
    }

    let mut cfg = CfgFunction::new("::top", "d1");
    let d1 = cfg.entry;
    let d2 = cfg.intern_block("d2");
    cfg.blocks.insert(d2, Block::new("d2"));
    let d3 = cfg.intern_block("d3");
    cfg.blocks.insert(d3, Block::new("d3"));
    let arm_a = cfg.intern_block("arm_a");
    cfg.blocks.insert(arm_a, Block::new("arm_a"));
    let arm_b = cfg.intern_block("arm_b");
    cfg.blocks.insert(arm_b, Block::new("arm_b"));
    let default = cfg.intern_block("default");
    cfg.blocks.insert(default, Block::new("default"));
    let switch_end = cfg.intern_block("switch_end_1");
    cfg.blocks.insert(switch_end, Block::new("switch_end_1"));
    cfg.blocks.get_mut(&d1).unwrap().terminator = Some(str_eq_branch("x", "a", arm_a, d2));
    cfg.blocks.get_mut(&d2).unwrap().terminator = Some(str_eq_branch("x", "b", arm_b, d3));
    cfg.blocks.get_mut(&d3).unwrap().terminator = Some(Terminator::Goto {
        target: default,
        span: None,
    });
    cfg.blocks
        .get_mut(&arm_a)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&arm_a).unwrap().terminator = Some(Terminator::Goto {
        target: switch_end,
        span: None,
    });
    cfg.blocks
        .get_mut(&arm_b)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&arm_b).unwrap().terminator = Some(Terminator::Goto {
        target: switch_end,
        span: None,
    });
    cfg.blocks.get_mut(&default).unwrap().terminator = Some(Terminator::Goto {
        target: switch_end,
        span: None,
    });
    cfg.blocks.get_mut(&switch_end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::JUMP_TABLE),
        "expected JUMP_TABLE opcode, got {ops:?}"
    );
}

#[test]
fn switch_glob_as_proc_tail_keeps_result_on_stack() {
    // Regression: a glob/regexp `switch` as a proc's
    // last command must leave the invoke result on TOS for the proc
    // return — emitting a statement-level POP underflows the stack.
    use tcl_compiler::cfg_builder::build_cfg_codegen;
    use tcl_compiler::lowering::lower_to_ir;

    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir("proc f {x} { switch -glob -- $x a* {set r 1} }", &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let f = asm.procedures.get("::f").expect("proc ::f");
    let ops: Vec<Op> = f.instructions.iter().map(|i| i.op).collect();
    // The arm bodies are subsumed by the runtime invoke, so the only
    // command is the `switch` invoke. Its result flows straight to the
    // proc return — there must be no POP dropping it.
    assert!(
        ops.contains(&Op::INVOKE_STK1) || ops.contains(&Op::INVOKE_STK4),
        "expected a generic switch invoke, got {ops:?}",
    );
    assert!(
        !ops.contains(&Op::POP),
        "glob switch as proc tail must not POP its result, got {ops:?}",
    );
}

#[test]
fn foreach_synthetic_ops_carry_no_source_span() {
    // Regression: foreach_step / foreach_end are synthetic
    // loop machinery with no Tcl source construct. The sticky statement span
    // must be cleared after the body so they serialise as null `range` in the
    // explorer asm view, rather than inheriting the last body statement's
    // span (which would render them as clickable ranges on that statement).
    use tcl_compiler::cfg_builder::build_cfg_codegen;
    use tcl_compiler::lowering::lower_to_ir;

    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir("foreach x {1 2 3} { set y $x }", &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let foreach_ops: Vec<_> = asm
        .top_level
        .instructions
        .iter()
        .filter(|i| matches!(i.op, Op::FOREACH_STEP | Op::FOREACH_END))
        .collect();
    assert!(
        !foreach_ops.is_empty(),
        "expected foreach_step/foreach_end to be emitted"
    );
    for inst in foreach_ops {
        assert_eq!(
            inst.source_span, None,
            "synthetic {:?} must not inherit a body statement's span",
            inst.op
        );
    }
}

#[test]
fn switch_glob_emits_generic_invoke_not_jump_table() {
    use tcl_compiler::cfg_builder::build_cfg_function;
    use tcl_compiler::ir::{SwitchArm, SwitchMode};

    let arm = |pat: &str| SwitchArm {
        pattern: pat.into(),
        pattern_span: sp(),
        body: Some(Script::from_statements(vec![Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }])),
        body_span: Some(sp()),
        fallthrough: false,
    };
    let make = |mode: SwitchMode| {
        Script::from_statements(vec![Statement::Switch {
            span: sp(),
            subject: "$x".into(),
            subject_span: sp(),
            arms: vec![arm("a*"), arm("b*")],
            default_body: None,
            default_span: None,
            mode,
            nocase: false,
            raw_args: vec!["$x".into(), "a* {set r 1} b* {set r 1}".into()],
            patterns_braced: true,
        }])
    };
    let registry = CommandRegistry::build_default();

    // Glob: generic `switch` invoke, never a jump table (glob patterns
    // are not exact string equality).
    let glob = build_cfg_function("::top", &make(SwitchMode::Glob), true);
    let ops: Vec<Op> = codegen_function(&glob, &[], false, &registry)
        .instructions
        .iter()
        .map(|i| i.op)
        .collect();
    assert!(
        !ops.contains(&Op::JUMP_TABLE),
        "glob switch must not emit a jump table, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::INVOKE_STK1) || ops.contains(&Op::INVOKE_STK4),
        "glob switch should emit a generic invoke, got {ops:?}"
    );

    // Exact still compiles to a real jump table.
    let exact = build_cfg_function("::top", &make(SwitchMode::Exact), true);
    let exact_ops: Vec<Op> = codegen_function(&exact, &[], false, &registry)
        .instructions
        .iter()
        .map(|i| i.op)
        .collect();
    assert!(
        exact_ops.contains(&Op::JUMP_TABLE),
        "exact switch should still emit a jump table, got {exact_ops:?}"
    );
}

#[test]
fn foreach_emits_native_opcodes() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let header = cfg.intern_block("foreach_header_1");
    cfg.blocks.insert(header, Block::new("foreach_header_1"));
    let body = cfg.intern_block("foreach_body_1");
    cfg.blocks.insert(body, Block::new("foreach_body_1"));
    let end = cfg.intern_block("foreach_end_1");
    cfg.blocks.insert(end, Block::new("foreach_end_1"));

    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    // foreach command is the single statement in the header block
    cfg.blocks
        .get_mut(&header)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "foreach".into(),
            canonical_command: None,
            args: vec!["${lst}".into()],
            defs: vec!["i".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Raw {
            text: "<foreach>".into(),
        },
        true_target: body,
        false_target: end,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            name_braced: false,
            value: "42".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::FOREACH_START),
        "expected FOREACH_START, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::FOREACH_STEP),
        "expected FOREACH_STEP, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::FOREACH_END),
        "expected FOREACH_END, got {ops:?}"
    );
}

#[test]
// One linear hand-built CFG fixture plus its opcode assertions; splitting the
// blocks across helpers would scatter the control-flow shape under test.
#[allow(clippy::too_many_lines)]
fn complex_foreach_body_emits_step_at_end() {
    // Build a foreach whose body is a branch (if condition → break).
    //   foreach_header_1 ─branch─→ foreach_body_1 (empty, if cond)
    //                   ─else──→  foreach_end_1
    //   foreach_body_1 ─branch(cond)─→ if_then_1 (break)
    //                              └→ if_end_1 (goto foreach_header_1)
    //   if_then_1 ─break command─→ foreach_end_1 (via loop_ctx)
    //   foreach_end_1 ─→ return
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let header = cfg.intern_block("foreach_header_1");
    cfg.blocks.insert(header, Block::new("foreach_header_1"));
    let body = cfg.intern_block("foreach_body_1");
    cfg.blocks.insert(body, Block::new("foreach_body_1"));
    let then = cfg.intern_block("if_then_1");
    cfg.blocks.insert(then, Block::new("if_then_1"));
    let if_end = cfg.intern_block("if_end_1");
    cfg.blocks.insert(if_end, Block::new("if_end_1"));
    let end = cfg.intern_block("foreach_end_1");
    cfg.blocks.insert(end, Block::new("foreach_end_1"));

    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });

    // foreach header has the foreach statement.
    cfg.blocks
        .get_mut(&header)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "foreach".into(),
            canonical_command: None,
            args: vec!["${lst}".into()],
            defs: vec!["i".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Raw {
            text: "<foreach>".into(),
        },
        true_target: body,
        false_target: end,
        span: None,
        condition_base: None,
    });

    // Complex body: empty, branch terminator
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Var {
            text: "$i".into(),
            name: "i".into(),
            start: 0,
            end: 2,
        },
        true_target: then,
        false_target: if_end,
        span: None,
        condition_base: None,
    });

    // if_then_1: a break statement
    cfg.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "break".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });

    // if_end_1: no-op, loops back
    cfg.blocks.get_mut(&if_end).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });

    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], false, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::FOREACH_START),
        "expected FOREACH_START, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::FOREACH_STEP),
        "expected FOREACH_STEP, got {ops:?}"
    );
    assert!(
        ops.contains(&Op::FOREACH_END),
        "expected FOREACH_END, got {ops:?}"
    );
    // The foreach_continue_N and foreach_break_N labels should be
    // present in the label table, indicating we emitted them.
    let has_continue_label = asm
        .labels
        .keys()
        .any(|k| k.starts_with("foreach_continue_"));
    let has_break_label = asm.labels.keys().any(|k| k.starts_with("foreach_break_"));
    assert!(has_continue_label, "expected foreach_continue_N label");
    assert!(has_break_label, "expected foreach_break_N label");
}

#[test]
fn while_in_proc_emits_start_cmd() {
    use tcl_compiler::expr_ast::ExprNode;

    let mut cfg = CfgFunction::new("::f", "entry_0");
    let entry = cfg.entry;
    let header = cfg.intern_block("while_header_1");
    cfg.blocks.insert(header, Block::new("while_header_1"));
    let body = cfg.intern_block("while_body_1");
    cfg.blocks.insert(body, Block::new("while_body_1"));
    let end = cfg.intern_block("while_end_1");
    cfg.blocks.insert(end, Block::new("while_end_1"));

    // Entry: a set statement, then goto while_header_1
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Var {
            text: "$i".into(),
            name: "i".into(),
            start: 0,
            end: 2,
        },
        true_target: body,
        false_target: end,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(Statement::Incr {
            span: sp(),
            name: "i".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        });
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let asm = codegen_function(&cfg, &[], true, &registry);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::START_CMD),
        "expected START_CMD for while loop in proc, got {ops:?}"
    );
}

// Modules

#[test]
fn codegen_module_with_no_procs() {
    let cfg_mod = CfgModule {
        top_level: toplevel_with(vec![]),
        procedures: HashMap::new(),
    };
    let ir_mod = IrModule {
        source: String::new(),
        top_level: Script::new(),
        procedures: HashMap::new(),
        methods: HashMap::new(),
        body_units: HashMap::new(),
        lambda_body_units: std::collections::BTreeSet::new(),
        redefined_procedures: HashSet::new(),
        redefined_methods: HashSet::new(),
        namespace_imports: Vec::new(),
        namespace_exports: Vec::new(),
        traced_commands: std::collections::BTreeSet::new(),
        has_dynamic_trace: false,
        traced_variables: std::collections::BTreeSet::new(),
        has_dynamic_variable_trace: false,
    };
    let registry = CommandRegistry::build_default();
    let asm = codegen_module(&cfg_mod, &ir_mod, &registry);
    assert_eq!(asm.top_level.name, "::top");
    assert!(asm.procedures.is_empty());
}
