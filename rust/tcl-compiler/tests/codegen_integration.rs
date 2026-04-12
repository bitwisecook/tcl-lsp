//! Integration tests for the codegen emitter pipeline.
//!
//! Builds small CFG fixtures and runs them through `codegen_function`
//! / `codegen_module` to verify the resulting `FunctionAsm` shape.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::collections::{HashMap, HashSet};

use tcl_compiler::cfg::{Block, CfgModule, Function as CfgFunction, Terminator};
use tcl_compiler::codegen::{codegen_function, codegen_module, Op};
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::{Module as IrModule, Script, Statement};
use tcl_lexer::Span;

fn sp() -> Span {
    Span::new(0, 0)
}

fn toplevel_with(statements: Vec<Statement>) -> CfgFunction {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    {
        let blk = cfg.blocks.get_mut("entry_0").unwrap();
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
    {
        let blk = cfg.blocks.get_mut("entry_0").unwrap();
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

// ---------------------------------------------------------------------------
// Top-level scripts
// ---------------------------------------------------------------------------

#[test]
fn empty_toplevel_has_done_or_return_imm() {
    let cfg = toplevel_with(vec![]);
    let asm = codegen_function(&cfg, &[], false);
    assert_eq!(asm.name, "::top");
    let last = asm.instructions.last().unwrap().op;
    assert!(matches!(last, Op::DONE | Op::RETURN_IMM));
}

#[test]
fn set_const_toplevel() {
    let cfg = toplevel_with(vec![Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "42".into(),
    }]);
    let asm = codegen_function(&cfg, &[], false);
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
            value: "1".into(),
        },
        Statement::AssignConst {
            span: sp(),
            name: "b".into(),
            value: "2".into(),
        },
    ]);
    let asm = codegen_function(&cfg, &[], false);
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
        args: vec!["hello".into()],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
    }]);
    let asm = codegen_function(&cfg, &[], false);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::INVOKE_STK1));
}

// ---------------------------------------------------------------------------
// Proc bodies
// ---------------------------------------------------------------------------

#[test]
fn empty_proc_pushes_empty_and_dones() {
    let cfg = proc_with("::foo", &[], vec![]);
    let asm = codegen_function(&cfg, &[], true);
    // The peephole pass collapses the trailing pop into done, so we
    // should end with DONE.
    assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn proc_return_param_loads_and_dones() {
    let mut cfg = CfgFunction::new("::f", "entry_0");
    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Return {
        value: Some("${x}".into()),
        span: None,
        expr: None,
        braced: false,
    });
    let asm = codegen_function(&cfg, &["x"], true);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::LOAD_SCALAR1));
    assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
    // The LVT should have x at slot 0
    assert_eq!(asm.lvt.entries()[0], "x");
}

// ---------------------------------------------------------------------------
// If / branching
// ---------------------------------------------------------------------------

#[test]
fn if_else_diamond_emits_conditional_jump() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    cfg.blocks.insert("if_then_1".into(), Block::new("if_then_1"));
    cfg.blocks.insert("if_else_1".into(), Block::new("if_else_1"));
    cfg.blocks.insert("if_end_1".into(), Block::new("if_end_1"));

    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        },
        true_target: "if_then_1".into(),
        false_target: "if_else_1".into(),
        span: None,
    });
    cfg.blocks
        .get_mut("if_then_1")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "1".into(),
        });
    cfg.blocks.get_mut("if_then_1").unwrap().terminator = Some(Terminator::Goto {
        target: "if_end_1".into(),
        span: None,
    });
    cfg.blocks
        .get_mut("if_else_1")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "2".into(),
        });
    cfg.blocks.get_mut("if_else_1").unwrap().terminator = Some(Terminator::Goto {
        target: "if_end_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("if_end_1").unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let asm = codegen_function(&cfg, &[], false);
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
    cfg.blocks.insert("if_then_1".into(), Block::new("if_then_1"));
    cfg.blocks.insert("if_else_1".into(), Block::new("if_else_1"));
    cfg.blocks.insert("if_end_1".into(), Block::new("if_end_1"));

    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: "if_then_1".into(),
        false_target: "if_else_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("if_then_1").unwrap().terminator = Some(Terminator::Goto {
        target: "if_end_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("if_else_1").unwrap().terminator = Some(Terminator::Goto {
        target: "if_end_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("if_end_1").unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let asm = codegen_function(&cfg, &[], false);
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

    fn str_eq_branch(var: &str, pat: &str, tt: &str, ft: &str) -> Terminator {
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
            true_target: tt.into(),
            false_target: ft.into(),
            span: None,
        }
    }

    let mut cfg = CfgFunction::new("::top", "d1");
    for b in ["d2", "d3", "arm_a", "arm_b", "default", "switch_end_1"] {
        cfg.blocks.insert(b.into(), Block::new(b));
    }
    cfg.blocks.get_mut("d1").unwrap().terminator = Some(str_eq_branch("x", "a", "arm_a", "d2"));
    cfg.blocks.get_mut("d2").unwrap().terminator = Some(str_eq_branch("x", "b", "arm_b", "d3"));
    cfg.blocks.get_mut("d3").unwrap().terminator = Some(Terminator::Goto {
        target: "default".into(),
        span: None,
    });
    cfg.blocks
        .get_mut("arm_a")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "1".into(),
        });
    cfg.blocks.get_mut("arm_a").unwrap().terminator = Some(Terminator::Goto {
        target: "switch_end_1".into(),
        span: None,
    });
    cfg.blocks
        .get_mut("arm_b")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "2".into(),
        });
    cfg.blocks.get_mut("arm_b").unwrap().terminator = Some(Terminator::Goto {
        target: "switch_end_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("default").unwrap().terminator = Some(Terminator::Goto {
        target: "switch_end_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("switch_end_1").unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let asm = codegen_function(&cfg, &[], false);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::JUMP_TABLE),
        "expected JUMP_TABLE opcode, got {ops:?}"
    );
}

#[test]
fn foreach_emits_native_opcodes() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    cfg.blocks
        .insert("foreach_header_1".into(), Block::new("foreach_header_1"));
    cfg.blocks
        .insert("foreach_body_1".into(), Block::new("foreach_body_1"));
    cfg.blocks
        .insert("foreach_end_1".into(), Block::new("foreach_end_1"));

    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Goto {
        target: "foreach_header_1".into(),
        span: None,
    });
    // foreach command is the single statement in the header block
    cfg.blocks
        .get_mut("foreach_header_1")
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "foreach".into(),
            args: vec!["${lst}".into()],
            defs: vec!["i".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });
    cfg.blocks.get_mut("foreach_header_1").unwrap().terminator =
        Some(Terminator::Branch {
            condition: ExprNode::Raw {
                text: "<foreach>".into(),
            },
            true_target: "foreach_body_1".into(),
            false_target: "foreach_end_1".into(),
            span: None,
        });
    cfg.blocks
        .get_mut("foreach_body_1")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "42".into(),
        });
    cfg.blocks.get_mut("foreach_body_1").unwrap().terminator =
        Some(Terminator::Goto {
            target: "foreach_header_1".into(),
            span: None,
        });
    cfg.blocks.get_mut("foreach_end_1").unwrap().terminator =
        Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

    let asm = codegen_function(&cfg, &[], false);
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
fn complex_foreach_body_emits_step_at_end() {
    // Build a foreach whose body is a branch (if condition → break).
    //   foreach_header_1 ─branch─→ foreach_body_1 (empty, if cond)
    //                   ─else──→  foreach_end_1
    //   foreach_body_1 ─branch(cond)─→ if_then_1 (break)
    //                              └→ if_end_1 (goto foreach_header_1)
    //   if_then_1 ─break command─→ foreach_end_1 (via loop_ctx)
    //   foreach_end_1 ─→ return
    let mut cfg = CfgFunction::new("::top", "entry_0");
    for b in [
        "foreach_header_1",
        "foreach_body_1",
        "if_then_1",
        "if_end_1",
        "foreach_end_1",
    ] {
        cfg.blocks.insert(b.into(), Block::new(b));
    }

    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Goto {
        target: "foreach_header_1".into(),
        span: None,
    });

    // foreach header has the foreach statement.
    cfg.blocks
        .get_mut("foreach_header_1")
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "foreach".into(),
            args: vec!["${lst}".into()],
            defs: vec!["i".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });
    cfg.blocks.get_mut("foreach_header_1").unwrap().terminator =
        Some(Terminator::Branch {
            condition: ExprNode::Raw {
                text: "<foreach>".into(),
            },
            true_target: "foreach_body_1".into(),
            false_target: "foreach_end_1".into(),
            span: None,
        });

    // Complex body: empty, branch terminator
    cfg.blocks.get_mut("foreach_body_1").unwrap().terminator =
        Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$i".into(),
                name: "i".into(),
                start: 0,
                end: 2,
            },
            true_target: "if_then_1".into(),
            false_target: "if_end_1".into(),
            span: None,
        });

    // if_then_1: a break statement
    cfg.blocks
        .get_mut("if_then_1")
        .unwrap()
        .statements
        .push(Statement::Call {
            span: sp(),
            command: "break".into(),
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });
    cfg.blocks.get_mut("if_then_1").unwrap().terminator = Some(Terminator::Goto {
        target: "foreach_header_1".into(),
        span: None,
    });

    // if_end_1: no-op, loops back
    cfg.blocks.get_mut("if_end_1").unwrap().terminator = Some(Terminator::Goto {
        target: "foreach_header_1".into(),
        span: None,
    });

    cfg.blocks.get_mut("foreach_end_1").unwrap().terminator =
        Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

    let asm = codegen_function(&cfg, &[], false);
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
    let has_break_label = asm
        .labels
        .keys()
        .any(|k| k.starts_with("foreach_break_"));
    assert!(has_continue_label, "expected foreach_continue_N label");
    assert!(has_break_label, "expected foreach_break_N label");
}

#[test]
fn while_in_proc_emits_start_cmd() {
    use tcl_compiler::expr_ast::ExprNode;

    let mut cfg = CfgFunction::new("::f", "entry_0");
    cfg.blocks
        .insert("while_header_1".into(), Block::new("while_header_1"));
    cfg.blocks
        .insert("while_body_1".into(), Block::new("while_body_1"));
    cfg.blocks
        .insert("while_end_1".into(), Block::new("while_end_1"));

    // Entry: a set statement, then goto while_header_1
    cfg.blocks
        .get_mut("entry_0")
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "i".into(),
            value: "0".into(),
        });
    cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Goto {
        target: "while_header_1".into(),
        span: None,
    });
    cfg.blocks.get_mut("while_header_1").unwrap().terminator =
        Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$i".into(),
                name: "i".into(),
                start: 0,
                end: 2,
            },
            true_target: "while_body_1".into(),
            false_target: "while_end_1".into(),
            span: None,
        });
    cfg.blocks
        .get_mut("while_body_1")
        .unwrap()
        .statements
        .push(Statement::Incr {
            span: sp(),
            name: "i".into(),
            amount: None,
            safe_on_uninit: false,
        });
    cfg.blocks.get_mut("while_body_1").unwrap().terminator =
        Some(Terminator::Goto {
            target: "while_header_1".into(),
            span: None,
        });
    cfg.blocks.get_mut("while_end_1").unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });

    let asm = codegen_function(&cfg, &[], true);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::START_CMD),
        "expected START_CMD for while loop in proc, got {ops:?}"
    );
}

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

#[test]
fn codegen_module_with_no_procs() {
    let cfg_mod = CfgModule {
        top_level: toplevel_with(vec![]),
        procedures: HashMap::new(),
    };
    let ir_mod = IrModule {
        top_level: Script::new(),
        procedures: HashMap::new(),
        methods: HashMap::new(),
        redefined_procedures: HashSet::new(),
    };
    let asm = codegen_module(&cfg_mod, &ir_mod);
    assert_eq!(asm.top_level.name, "::top");
    assert!(asm.procedures.is_empty());
}
